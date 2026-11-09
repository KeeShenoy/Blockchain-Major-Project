//! Issuer: generates a credential batch, its Rescue Merkle root, and one holder's private witness.
//!
//! The root is public and goes to Ethereum. The witness goes only to the holder, and must never be
//! published.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use merkle_core::{validate_depth, Hash, Rescue128, RootFile, Witness, WitnessFile};
use rand::{rngs::OsRng, rngs::StdRng, RngCore, SeedableRng};
use winterfell::{crypto::MerkleTree, math::fields::f128::BaseElement};

const USAGE: &str = "usage: issuer <depth> <holder_index> [--seed <u64>] [--output <dir>]
       issuer --synthetic <depth> --seed <u64> [--output <dir>]

--seed makes issuance reproducible and is for benchmark fixtures only.
--synthetic writes a benchmark witness with a generated path and no real batch,
for depths whose tree is too large to build.";

/// Largest depth built as a real tree: 2^24 leaves already need over 1 GiB of nodes.
const MAX_MATERIALIZED_DEPTH: usize = 24;

fn random_credential(rng: &mut impl RngCore) -> [BaseElement; 2] {
    [
        BaseElement::new(rng.next_u64() as u128),
        BaseElement::new(rng.next_u64() as u128),
    ]
}

fn issue(depth: usize, holder_index: usize, rng: &mut impl RngCore) -> Result<Witness, String> {
    validate_depth(depth).map_err(|error| error.to_string())?;
    if depth > MAX_MATERIALIZED_DEPTH {
        return Err(format!(
            "depth {depth} is too large to build a real tree (maximum {MAX_MATERIALIZED_DEPTH})"
        ));
    }
    let leaf_count = 1usize << depth;
    if holder_index >= leaf_count {
        return Err(format!(
            "holder index {holder_index} is outside a batch of {leaf_count} credentials"
        ));
    }

    let mut holder_value = None;
    let leaves: Vec<Hash> = (0..leaf_count)
        .map(|index| {
            let value = random_credential(rng);
            if index == holder_index {
                holder_value = Some(value);
            }
            Rescue128::digest(&value)
        })
        .collect();
    let tree = MerkleTree::<Rescue128>::new(leaves).map_err(|error| error.to_string())?;
    let (_, path) = tree
        .prove(holder_index)
        .map_err(|error| error.to_string())?;
    let value = holder_value.expect("holder_index is within the batch");

    let witness = Witness::new(value, holder_index, path).map_err(|error| error.to_string())?;
    if witness.root() != tree.root().to_elements() {
        return Err("internal error: witness does not resolve to the tree root".into());
    }
    Ok(witness)
}

/// Writes the witness readable only by its owner, since it reveals the holder's credential.
fn write_private(path: &Path, contents: String) -> std::io::Result<()> {
    fs::write(path, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

struct Arguments {
    positional: Vec<String>,
    seed: Option<u64>,
    output: PathBuf,
    synthetic: bool,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut parsed = Arguments {
        positional: Vec::new(),
        seed: None,
        output: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("output"),
        synthetic: false,
    };
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--seed" => {
                let value = arguments.next().ok_or("--seed requires a value")?;
                parsed.seed = Some(
                    value
                        .parse()
                        .map_err(|_| "--seed must be an unsigned integer")?,
                );
            }
            "--output" => {
                parsed.output = PathBuf::from(arguments.next().ok_or("--output requires a value")?);
            }
            "--synthetic" => parsed.synthetic = true,
            _ if argument.starts_with("--") => return Err(format!("unknown option {argument}")),
            _ => parsed.positional.push(argument),
        }
    }
    Ok(parsed)
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments()?;
    let parse_number = |text: &String, name: &str| {
        text.parse::<usize>()
            .map_err(|_| format!("{name} must be a non-negative integer"))
    };

    let witness = match (arguments.synthetic, arguments.positional.as_slice()) {
        (true, [depth]) => {
            let seed = arguments.seed.ok_or("--synthetic requires --seed")?;
            Witness::synthetic(parse_number(depth, "depth")?, seed)
                .map_err(|error| error.to_string())?
        }
        (false, [depth, holder_index]) => {
            let depth = parse_number(depth, "depth")?;
            let holder_index = parse_number(holder_index, "holder_index")?;
            match arguments.seed {
                Some(seed) => issue(depth, holder_index, &mut StdRng::seed_from_u64(seed))?,
                None => issue(depth, holder_index, &mut OsRng)?,
            }
        }
        _ => return Err(USAGE.into()),
    };

    let root = RootFile::from_elements(witness.root());
    let witness_file = WitnessFile::from(&witness);
    let json_error = |error: serde_json::Error| error.to_string();
    let io_error = |error: std::io::Error| error.to_string();

    fs::create_dir_all(&arguments.output).map_err(io_error)?;
    fs::write(
        arguments.output.join("root.json"),
        serde_json::to_string_pretty(&root).map_err(json_error)?,
    )
    .map_err(io_error)?;
    write_private(
        &arguments
            .output
            .join(format!("witness_{}.json", witness.index)),
        serde_json::to_string_pretty(&witness_file).map_err(json_error)?,
    )
    .map_err(io_error)?;

    println!("root_limb0: {}", root.root_limb0);
    println!("root_limb1: {}", root.root_limb1);
    println!("witness_path_length: {}", witness.depth());
    println!("holder_index: {}", witness.index);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("issuer: {message}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use merkle_core::root_from_path;

    use super::*;

    #[test]
    fn issued_witness_reconstructs_root() {
        let witness = issue(3, 5, &mut OsRng).unwrap();
        assert_eq!(witness.depth(), 3);
        assert_eq!(witness.index, 5);
        assert_eq!(
            root_from_path(witness.value, &witness.path, witness.index).to_elements(),
            witness.root()
        );
    }

    #[test]
    fn witness_serialization_round_trip() {
        let witness = issue(3, 1, &mut OsRng).unwrap();
        let encoded = serde_json::to_string(&WitnessFile::from(&witness)).unwrap();
        let decoded: WitnessFile = serde_json::from_str(&encoded).unwrap();
        assert_eq!(Witness::try_from(&decoded).unwrap().root(), witness.root());
    }

    #[test]
    fn tampered_witness_does_not_reconstruct_root() {
        let witness = issue(3, 2, &mut OsRng).unwrap();
        let mut file = WitnessFile::from(&witness);
        file.path[0].limb0 = "0".into();
        let tampered = Witness::try_from(&file).unwrap();
        assert_ne!(tampered.root(), witness.root());
    }

    #[test]
    fn seeded_issuance_is_reproducible() {
        let a = issue(3, 2, &mut StdRng::seed_from_u64(7)).unwrap();
        let b = issue(3, 2, &mut StdRng::seed_from_u64(7)).unwrap();
        assert_eq!(a.root(), b.root());
    }

    #[test]
    fn invalid_batches_are_rejected() {
        assert!(issue(5, 0, &mut OsRng).is_err()); // trace length would not be a power of two
        assert!(issue(3, 8, &mut OsRng).is_err()); // holder outside the batch
        assert!(issue(31, 0, &mut OsRng).is_err()); // too large to materialize
    }
}
