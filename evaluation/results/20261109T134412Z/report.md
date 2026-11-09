# Benchmark report 20261109T134412Z

Revision ccd0d2b-dirty, Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz, kernel 6.18.54.1-microsoft-standard-WSL2, rustc 1.97.1 (8bab26f4f 2026-07-14). Depth 7, medians.

| Metric | Original baseline | Original folded | Previous folded | Current optimized |
|---|---:|---:|---:|---:|
| Proving, trace + prove, warm (µs) | 2,873.8 | 2,727.5 | 2,778.7 | 2,411.5 |
| Proving, prove only, warm (µs) | 2,012.1 | 1,970.3 | 2,025.0 | 1,806.7 |
| Trace build, warm (µs) | 861.7 | 756.5 | 756.8 | 606.0 |
| Verification, 30 witnesses (µs) | 229.6 | 414.2 | 397.3 | 131.1 |
| Proof size, 30 witnesses (B) | 15,799 | 17,344 | 17,056 | 12,824 |
| Peak heap during proving (B) | 235,519 | 237,019 | 170,015 | 150,770 |
| Peak RSS after proving (B) | 3,665,920 | 3,674,112 | 3,297,280 | 3,342,336 |

## Paired comparisons

An interval containing zero means no established difference.

| Comparison | Metric | Experiment | Pairs | Control | Treatment | Change | 95% CI |
|---|---|---|---:|---:|---:|---:|---|
| optimized vs original baseline | prove_end_to_end_ns | main | 15 | 2,873,772 | 2,411,513 | -14.0% | [-17.4%, -13.5%] |
| optimized vs original baseline | prove_warm_median_ns | main | 15 | 2,012,097 | 1,806,654 | -7.8% | [-12.1%, -7.3%] |
| optimized vs original baseline | prove_first_ns | main | 15 | 2,164,151 | 1,947,207 | -10.8% | [-17.3%, -6.1%] |
| optimized vs original baseline | trace_build_warm_median_ns | main | 15 | 861,675 | 605,982 | -28.4% | [-29.7%, -27.5%] |
| optimized vs original baseline | verify_median_ns | main | 15 | 226,721 | 126,391 | -41.5% | [-46.1%, -40.9%] |
| optimized vs original baseline | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,342,336 | -8.8% | [-9.7%, -8.5%] |
| optimized vs original baseline | peak_rss_process_bytes | main | 15 | 3,977,216 | 3,907,584 | -2.6% | [-3.2%, -1.3%] |
| optimized vs original baseline | proof_size_bytes | main | 15 | 15,542 | 12,693 | -18.3% | [-18.3%, -18.3%] |
| optimized vs original baseline | proof_size_bytes | witness | 30 | 15,799 | 12,824 | -18.2% | [-19.7%, -16.9%] |
| optimized vs original baseline | verify_median_ns | witness | 30 | 229,590 | 131,144 | -42.2% | [-44.7%, -40.4%] |
| optimized vs original folded | prove_end_to_end_ns | main | 15 | 2,727,539 | 2,411,513 | -11.2% | [-19.1%, -9.6%] |
| optimized vs original folded | prove_warm_median_ns | main | 15 | 1,970,325 | 1,806,654 | -7.9% | [-16.2%, -6.3%] |
| optimized vs original folded | prove_first_ns | main | 15 | 2,182,527 | 1,947,207 | -10.4% | [-17.4%, -6.2%] |
| optimized vs original folded | trace_build_warm_median_ns | main | 15 | 756,465 | 605,982 | -20.0% | [-26.5%, -17.5%] |
| optimized vs original folded | verify_median_ns | main | 15 | 432,615 | 126,391 | -70.9% | [-77.1%, -68.4%] |
| optimized vs original folded | peak_rss_after_prove_bytes | main | 15 | 3,674,112 | 3,342,336 | -9.1% | [-10.0%, -8.1%] |
| optimized vs original folded | peak_rss_process_bytes | main | 15 | 4,009,984 | 3,907,584 | -2.6% | [-3.3%, -0.9%] |
| optimized vs original folded | proof_size_bytes | main | 15 | 18,194 | 12,693 | -30.2% | [-30.2%, -30.2%] |
| optimized vs original folded | proof_size_bytes | witness | 30 | 17,344 | 12,824 | -25.6% | [-27.4%, -23.9%] |
| optimized vs original folded | verify_median_ns | witness | 30 | 414,168 | 131,144 | -66.8% | [-71.2%, -64.0%] |
| optimized vs previous folded | prove_end_to_end_ns | main | 15 | 2,778,693 | 2,411,513 | -13.7% | [-19.4%, -11.0%] |
| optimized vs previous folded | prove_warm_median_ns | main | 15 | 2,024,966 | 1,806,654 | -11.4% | [-16.4%, -8.5%] |
| optimized vs previous folded | prove_first_ns | main | 15 | 2,125,506 | 1,947,207 | -8.2% | [-19.5%, -5.1%] |
| optimized vs previous folded | trace_build_warm_median_ns | main | 15 | 756,773 | 605,982 | -19.9% | [-25.2%, -17.5%] |
| optimized vs previous folded | verify_median_ns | main | 15 | 400,838 | 126,391 | -68.2% | [-76.8%, -64.8%] |
| optimized vs previous folded | peak_rss_after_prove_bytes | main | 15 | 3,297,280 | 3,342,336 | +1.4% | [+1.1%, +2.5%] |
| optimized vs previous folded | peak_rss_process_bytes | main | 15 | 3,837,952 | 3,907,584 | +1.2% | [+0.4%, +4.4%] |
| optimized vs previous folded | proof_size_bytes | main | 15 | 17,393 | 12,693 | -27.0% | [-27.0%, -27.0%] |
| optimized vs previous folded | proof_size_bytes | witness | 30 | 17,056 | 12,824 | -23.6% | [-26.8%, -21.8%] |
| optimized vs previous folded | verify_median_ns | witness | 30 | 397,285 | 131,144 | -67.4% | [-69.4%, -60.4%] |
| optimized vs previous split | prove_end_to_end_ns | main | 15 | 2,728,912 | 2,411,513 | -12.7% | [-16.8%, -9.0%] |
| optimized vs previous split | prove_warm_median_ns | main | 15 | 1,987,306 | 1,806,654 | -10.2% | [-14.7%, -7.4%] |
| optimized vs previous split | prove_first_ns | main | 15 | 2,135,590 | 1,947,207 | -11.4% | [-15.7%, -7.6%] |
| optimized vs previous split | trace_build_warm_median_ns | main | 15 | 736,356 | 605,982 | -21.4% | [-22.5%, -13.6%] |
| optimized vs previous split | verify_median_ns | main | 15 | 229,064 | 126,391 | -44.6% | [-48.7%, -41.2%] |
| optimized vs previous split | peak_rss_after_prove_bytes | main | 15 | 3,297,280 | 3,342,336 | +1.4% | [+0.9%, +2.0%] |
| optimized vs previous split | peak_rss_process_bytes | main | 15 | 3,895,296 | 3,907,584 | +0.2% | [-0.6%, +1.3%] |
| optimized vs previous split | proof_size_bytes | main | 15 | 16,793 | 12,693 | -24.4% | [-24.4%, -24.4%] |
| current folded vs original folded | prove_end_to_end_ns | main | 15 | 2,727,539 | 2,424,256 | -11.8% | [-18.7%, -8.5%] |
| current folded vs original folded | prove_warm_median_ns | main | 15 | 1,970,325 | 1,816,508 | -8.8% | [-15.6%, -4.4%] |
| current folded vs original folded | prove_first_ns | main | 15 | 2,182,527 | 1,909,520 | -11.2% | [-12.3%, -9.6%] |
| current folded vs original folded | trace_build_warm_median_ns | main | 15 | 756,465 | 603,746 | -19.8% | [-26.9%, -18.3%] |
| current folded vs original folded | verify_median_ns | main | 15 | 432,615 | 404,450 | -6.3% | [-15.2%, -2.0%] |
| current folded vs original folded | peak_rss_after_prove_bytes | main | 15 | 3,674,112 | 3,354,624 | -8.6% | [-9.6%, -8.2%] |
| current folded vs original folded | peak_rss_process_bytes | main | 15 | 4,009,984 | 3,973,120 | -1.4% | [-2.6%, -0.9%] |
| current folded vs original folded | proof_size_bytes | main | 15 | 18,194 | 17,393 | -4.4% | [-4.4%, -4.4%] |
| current folded vs original folded | proof_size_bytes | witness | 30 | 17,344 | 17,056 | -1.1% | [-2.8%, +1.3%] |
| current folded vs original folded | verify_median_ns | witness | 30 | 414,168 | 386,289 | -3.4% | [-10.6%, +0.8%] |
| current baseline vs original baseline | prove_end_to_end_ns | main | 15 | 2,873,772 | 2,295,913 | -18.1% | [-19.9%, -12.5%] |
| current baseline vs original baseline | prove_warm_median_ns | main | 15 | 2,012,097 | 1,710,517 | -11.7% | [-14.7%, -7.1%] |
| current baseline vs original baseline | prove_first_ns | main | 15 | 2,164,151 | 1,932,438 | -13.7% | [-16.6%, -4.6%] |
| current baseline vs original baseline | trace_build_warm_median_ns | main | 15 | 861,675 | 589,503 | -30.2% | [-32.4%, -25.4%] |
| current baseline vs original baseline | verify_median_ns | main | 15 | 226,721 | 226,920 | +1.4% | [-0.4%, +5.2%] |
| current baseline vs original baseline | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,416,064 | -6.6% | [-7.9%, -5.3%] |
| current baseline vs original baseline | peak_rss_process_bytes | main | 15 | 3,977,216 | 3,940,352 | -0.7% | [-2.9%, +0.0%] |
| current baseline vs original baseline | proof_size_bytes | main | 15 | 15,542 | 16,793 | +8.0% | [+8.0%, +8.0%] |
| current baseline vs original baseline | proof_size_bytes | witness | 30 | 15,799 | 15,860 | -0.0% | [-3.2%, +2.0%] |
| current baseline vs original baseline | verify_median_ns | witness | 30 | 229,590 | 220,574 | -6.5% | [-9.4%, -1.9%] |
| folded vs baseline (original) | prove_end_to_end_ns | main | 15 | 2,873,772 | 2,727,539 | -4.2% | [-5.9%, -1.0%] |
| folded vs baseline (original) | prove_warm_median_ns | main | 15 | 2,012,097 | 1,970,325 | -0.5% | [-2.7%, +2.4%] |
| folded vs baseline (original) | prove_first_ns | main | 15 | 2,164,151 | 2,182,527 | +2.4% | [-3.8%, +4.5%] |
| folded vs baseline (original) | trace_build_warm_median_ns | main | 15 | 861,675 | 756,465 | -12.9% | [-13.8%, -9.0%] |
| folded vs baseline (original) | verify_median_ns | main | 15 | 226,721 | 432,615 | +94.0% | [+90.1%, +97.4%] |
| folded vs baseline (original) | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,674,112 | +0.2% | [-0.1%, +0.6%] |
| folded vs baseline (original) | peak_rss_process_bytes | main | 15 | 3,977,216 | 4,009,984 | +0.6% | [-0.4%, +2.5%] |
| folded vs baseline (original) | proof_size_bytes | main | 15 | 15,542 | 18,194 | +17.1% | [+17.1%, +17.1%] |
| folded vs baseline (original) | proof_size_bytes | witness | 30 | 15,799 | 17,344 | +9.6% | [+7.1%, +11.6%] |
| folded vs baseline (original) | verify_median_ns | witness | 30 | 229,590 | 414,168 | +77.9% | [+73.9%, +89.4%] |
| folded vs baseline (current) | prove_end_to_end_ns | main | 15 | 2,295,913 | 2,424,256 | +1.7% | [-3.6%, +5.6%] |
| folded vs baseline (current) | prove_warm_median_ns | main | 15 | 1,710,517 | 1,816,508 | +2.3% | [-3.7%, +6.3%] |
| folded vs baseline (current) | prove_first_ns | main | 15 | 1,932,438 | 1,909,520 | -0.7% | [-3.2%, +1.9%] |
| folded vs baseline (current) | trace_build_warm_median_ns | main | 15 | 589,503 | 603,746 | -0.6% | [-3.3%, +3.2%] |
| folded vs baseline (current) | verify_median_ns | main | 15 | 226,920 | 404,450 | +75.7% | [+70.3%, +83.0%] |
| folded vs baseline (current) | peak_rss_after_prove_bytes | main | 15 | 3,416,064 | 3,354,624 | -2.3% | [-3.4%, -0.5%] |
| folded vs baseline (current) | peak_rss_process_bytes | main | 15 | 3,940,352 | 3,973,120 | +1.5% | [-1.0%, +2.2%] |
| folded vs baseline (current) | proof_size_bytes | main | 15 | 16,793 | 17,393 | +3.6% | [+3.6%, +3.6%] |
| folded vs baseline (current) | proof_size_bytes | witness | 30 | 15,860 | 17,056 | +8.7% | [+4.9%, +9.9%] |
| folded vs baseline (current) | verify_median_ns | witness | 30 | 220,574 | 386,289 | +76.9% | [+67.8%, +87.2%] |
| optimized vs folded (current) | prove_end_to_end_ns | main | 15 | 2,424,256 | 2,411,513 | -0.5% | [-1.5%, +1.8%] |
| optimized vs folded (current) | prove_warm_median_ns | main | 15 | 1,816,508 | 1,806,654 | -0.7% | [-2.6%, +1.3%] |
| optimized vs folded (current) | prove_first_ns | main | 15 | 1,909,520 | 1,947,207 | -1.0% | [-4.2%, +1.6%] |
| optimized vs folded (current) | trace_build_warm_median_ns | main | 15 | 603,746 | 605,982 | +0.4% | [-1.7%, +4.1%] |
| optimized vs folded (current) | verify_median_ns | main | 15 | 404,450 | 126,391 | -67.9% | [-70.9%, -66.5%] |
| optimized vs folded (current) | peak_rss_after_prove_bytes | main | 15 | 3,354,624 | 3,342,336 | -0.6% | [-1.2%, +1.0%] |
| optimized vs folded (current) | peak_rss_process_bytes | main | 15 | 3,973,120 | 3,907,584 | -0.8% | [-3.2%, +0.5%] |
| optimized vs folded (current) | proof_size_bytes | main | 15 | 17,393 | 12,693 | -27.0% | [-27.0%, -27.0%] |
| optimized vs folded (current) | proof_size_bytes | witness | 30 | 17,056 | 12,824 | -23.6% | [-26.8%, -21.8%] |
| optimized vs folded (current) | verify_median_ns | witness | 30 | 386,289 | 131,144 | -65.9% | [-71.0%, -62.6%] |
| optimized vs baseline (current) | prove_end_to_end_ns | main | 15 | 2,295,913 | 2,411,513 | +2.4% | [-4.1%, +5.6%] |
| optimized vs baseline (current) | prove_warm_median_ns | main | 15 | 1,710,517 | 1,806,654 | +3.1% | [-4.0%, +7.6%] |
| optimized vs baseline (current) | prove_first_ns | main | 15 | 1,932,438 | 1,947,207 | -1.8% | [-6.1%, +3.5%] |
| optimized vs baseline (current) | trace_build_warm_median_ns | main | 15 | 589,503 | 605,982 | +0.2% | [-3.1%, +4.0%] |
| optimized vs baseline (current) | verify_median_ns | main | 15 | 226,920 | 126,391 | -45.7% | [-47.7%, -40.9%] |
| optimized vs baseline (current) | peak_rss_after_prove_bytes | main | 15 | 3,416,064 | 3,342,336 | -2.5% | [-4.0%, -1.6%] |
| optimized vs baseline (current) | peak_rss_process_bytes | main | 15 | 3,940,352 | 3,907,584 | -0.4% | [-1.1%, +0.9%] |
| optimized vs baseline (current) | proof_size_bytes | main | 15 | 16,793 | 12,693 | -24.4% | [-24.4%, -24.4%] |
| optimized vs baseline (current) | proof_size_bytes | witness | 30 | 15,860 | 12,824 | -18.0% | [-19.0%, -16.3%] |
| optimized vs baseline (current) | verify_median_ns | witness | 30 | 220,574 | 131,144 | -40.3% | [-41.0%, -36.6%] |
| storage: compact vs Winterfell, FRI 8 | prove_end_to_end_ns | main | 15 | 2,295,913 | 2,406,137 | +2.2% | [-1.5%, +3.7%] |
| storage: compact vs Winterfell, FRI 8 | prove_warm_median_ns | main | 15 | 1,710,517 | 1,806,280 | +3.0% | [-0.5%, +4.6%] |
| storage: compact vs Winterfell, FRI 8 | prove_first_ns | main | 15 | 1,932,438 | 1,921,488 | +0.5% | [-3.5%, +3.6%] |
| storage: compact vs Winterfell, FRI 8 | trace_build_warm_median_ns | main | 15 | 589,503 | 598,866 | -0.2% | [-2.5%, +0.7%] |
| storage: compact vs Winterfell, FRI 8 | verify_median_ns | main | 15 | 226,920 | 232,187 | +0.0% | [-2.9%, +2.6%] |
| storage: compact vs Winterfell, FRI 8 | peak_rss_after_prove_bytes | main | 15 | 3,416,064 | 3,342,336 | -2.4% | [-4.1%, -1.3%] |
| storage: compact vs Winterfell, FRI 8 | peak_rss_process_bytes | main | 15 | 3,940,352 | 3,911,680 | -1.0% | [-1.4%, +0.5%] |
| storage: compact vs Winterfell, FRI 8 | proof_size_bytes | main | 15 | 16,793 | 16,793 | +0.0% | [+0.0%, +0.0%] |
| storage: compact vs Winterfell, FRI 8 | proof_size_bytes | witness | 30 | 15,860 | 15,860 | +0.0% | [+0.0%, +0.0%] |
| storage: compact vs Winterfell, FRI 8 | verify_median_ns | witness | 30 | 220,574 | 218,982 | -0.2% | [-0.9%, +0.3%] |
| FRI shape: optimized vs split | prove_end_to_end_ns | main | 15 | 2,406,137 | 2,411,513 | -0.6% | [-2.4%, +3.9%] |
| FRI shape: optimized vs split | prove_warm_median_ns | main | 15 | 1,806,280 | 1,806,654 | -1.5% | [-2.9%, +4.3%] |
| FRI shape: optimized vs split | prove_first_ns | main | 15 | 1,921,488 | 1,947,207 | -1.3% | [-5.7%, +3.1%] |
| FRI shape: optimized vs split | trace_build_warm_median_ns | main | 15 | 598,866 | 605,982 | +2.2% | [-0.9%, +3.9%] |
| FRI shape: optimized vs split | verify_median_ns | main | 15 | 232,187 | 126,391 | -45.6% | [-47.5%, -41.4%] |
| FRI shape: optimized vs split | peak_rss_after_prove_bytes | main | 15 | 3,342,336 | 3,342,336 | -0.1% | [-0.9%, +0.4%] |
| FRI shape: optimized vs split | peak_rss_process_bytes | main | 15 | 3,911,680 | 3,907,584 | +0.2% | [-0.5%, +0.8%] |
| FRI shape: optimized vs split | proof_size_bytes | main | 15 | 16,793 | 12,693 | -24.4% | [-24.4%, -24.4%] |
| FRI shape: optimized vs split | proof_size_bytes | witness | 30 | 15,860 | 12,824 | -18.0% | [-19.0%, -16.3%] |
| FRI shape: optimized vs split | verify_median_ns | witness | 30 | 218,982 | 131,144 | -39.3% | [-41.4%, -36.8%] |
| Rescue arithmetic: exact routines vs field operators | prove_end_to_end_ns | main | 15 | 2,775,415 | 2,411,513 | -14.0% | [-15.2%, -10.7%] |
| Rescue arithmetic: exact routines vs field operators | prove_warm_median_ns | main | 15 | 2,018,342 | 1,806,654 | -11.8% | [-13.4%, -7.1%] |
| Rescue arithmetic: exact routines vs field operators | prove_first_ns | main | 15 | 2,205,245 | 1,947,207 | -11.3% | [-18.0%, -7.3%] |
| Rescue arithmetic: exact routines vs field operators | trace_build_warm_median_ns | main | 15 | 748,996 | 605,982 | -20.0% | [-22.4%, -18.9%] |
| Rescue arithmetic: exact routines vs field operators | verify_median_ns | main | 15 | 129,013 | 126,391 | -0.8% | [-4.0%, +0.5%] |
| Rescue arithmetic: exact routines vs field operators | peak_rss_after_prove_bytes | main | 15 | 3,284,992 | 3,342,336 | +1.5% | [+1.1%, +1.9%] |
| Rescue arithmetic: exact routines vs field operators | peak_rss_process_bytes | main | 15 | 3,878,912 | 3,907,584 | +0.6% | [+0.0%, +3.2%] |
| Rescue arithmetic: exact routines vs field operators | proof_size_bytes | main | 15 | 12,693 | 12,693 | +0.0% | [+0.0%, +0.0%] |
| trade-off: Blake3-192 vs Blake3-256 | proof_size_bytes | witness | 30 | 12,824 | 11,440 | -10.4% | [-14.3%, -9.2%] |
| trade-off: Blake3-192 vs Blake3-256 | verify_median_ns | witness | 30 | 131,144 | 134,320 | +0.8% | [-4.1%, +4.0%] |
| build: x86-64-v3 vs portable | prove_end_to_end_ns | main | 15 | 2,411,513 | 2,334,954 | -4.9% | [-9.0%, +0.2%] |
| build: x86-64-v3 vs portable | prove_warm_median_ns | main | 15 | 1,806,654 | 1,761,561 | -4.3% | [-8.0%, +1.7%] |
| build: x86-64-v3 vs portable | prove_first_ns | main | 15 | 1,947,207 | 1,908,556 | -3.3% | [-9.8%, +6.4%] |
| build: x86-64-v3 vs portable | trace_build_warm_median_ns | main | 15 | 605,982 | 565,735 | -6.7% | [-9.1%, -4.1%] |
| build: x86-64-v3 vs portable | verify_median_ns | main | 15 | 126,391 | 124,610 | -0.9% | [-4.6%, +0.2%] |
| build: x86-64-v3 vs portable | peak_rss_after_prove_bytes | main | 15 | 3,342,336 | 3,350,528 | +0.5% | [-0.4%, +1.5%] |
| build: x86-64-v3 vs portable | peak_rss_process_bytes | main | 15 | 3,907,584 | 3,956,736 | +0.2% | [-0.3%, +2.2%] |
| build: x86-64-v3 vs portable | proof_size_bytes | main | 15 | 12,693 | 12,693 | +0.0% | [+0.0%, +0.0%] |
| build: static vs dynamic | prove_end_to_end_ns | main | 15 | 2,411,513 | 2,359,282 | -2.9% | [-6.2%, +0.1%] |
| build: static vs dynamic | prove_warm_median_ns | main | 15 | 1,806,654 | 1,769,433 | -3.0% | [-6.2%, +0.1%] |
| build: static vs dynamic | prove_first_ns | main | 15 | 1,947,207 | 1,877,020 | -1.5% | [-9.0%, +9.5%] |
| build: static vs dynamic | trace_build_warm_median_ns | main | 15 | 605,982 | 587,546 | -2.1% | [-3.4%, +0.0%] |
| build: static vs dynamic | verify_median_ns | main | 15 | 126,391 | 125,512 | -1.4% | [-6.7%, -0.2%] |
| build: static vs dynamic | peak_rss_after_prove_bytes | main | 15 | 3,342,336 | 2,236,416 | -33.2% | [-33.7%, -32.7%] |
| build: static vs dynamic | peak_rss_process_bytes | main | 15 | 3,907,584 | 2,523,136 | -35.4% | [-37.4%, -34.9%] |
| build: static vs dynamic | proof_size_bytes | main | 15 | 12,693 | 12,693 | +0.0% | [+0.0%, +0.0%] |
| heap: optimized vs original baseline | heap_peak_prove_bytes | heap | 3 | 235,519 | 150,770 | -36.0% | [-36.0%, -36.0%] |
| heap: optimized vs original folded | heap_peak_prove_bytes | heap | 3 | 237,019 | 150,770 | -36.4% | [-36.4%, -36.4%] |
| heap: optimized vs previous compact | heap_peak_prove_bytes | heap | 3 | 170,015 | 150,770 | -11.3% | [-11.3%, -11.3%] |
| heap: folded vs folded with Winterfell storage | heap_peak_prove_bytes | heap | 3 | 236,543 | 156,135 | -34.0% | [-34.0%, -34.0%] |
| depth 3: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 1,550,067 | 1,240,756 | -20.3% | [-24.2%, -15.9%] |
| depth 3: optimized vs original baseline | verify_median_ns | scaling | 5 | 115,992 | 103,690 | -10.6% | [-120.7%, -6.7%] |
| depth 3: optimized vs original baseline | proof_size_bytes | scaling | 5 | 11,225 | 10,323 | -8.0% | [-8.0%, -8.0%] |
| depth 3: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,567,616 | 3,256,320 | -8.7% | [-10.0%, -6.1%] |
| depth 3: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 1,449,990 | 1,240,756 | -13.5% | [-20.3%, -9.8%] |
| depth 3: optimized vs original folded | verify_median_ns | scaling | 5 | 119,702 | 103,690 | -13.4% | [-16.4%, -9.8%] |
| depth 3: optimized vs original folded | proof_size_bytes | scaling | 5 | 11,291 | 10,323 | -8.6% | [-8.6%, -8.6%] |
| depth 3: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 3,567,616 | 3,256,320 | -7.7% | [-10.3%, -6.3%] |
| depth 7: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 2,894,319 | 2,363,082 | -16.8% | [-18.5%, -16.3%] |
| depth 7: optimized vs original baseline | verify_median_ns | scaling | 5 | 230,616 | 125,755 | -42.9% | [-48.2%, -42.5%] |
| depth 7: optimized vs original baseline | proof_size_bytes | scaling | 5 | 15,542 | 12,693 | -18.3% | [-18.3%, -18.3%] |
| depth 7: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,674,112 | 3,313,664 | -9.6% | [-9.8%, -9.0%] |
| depth 7: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 2,815,234 | 2,363,082 | -15.3% | [-18.7%, -13.1%] |
| depth 7: optimized vs original folded | verify_median_ns | scaling | 5 | 445,610 | 125,755 | -72.1% | [-74.1%, -68.2%] |
| depth 7: optimized vs original folded | proof_size_bytes | scaling | 5 | 18,194 | 12,693 | -30.2% | [-30.2%, -30.2%] |
| depth 7: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 3,674,112 | 3,313,664 | -9.7% | [-10.4%, -8.7%] |
| depth 15: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 5,978,936 | 4,710,814 | -20.1% | [-40.4%, -13.6%] |
| depth 15: optimized vs original baseline | verify_median_ns | scaling | 5 | 295,840 | 171,086 | -40.4% | [-63.6%, -29.8%] |
| depth 15: optimized vs original baseline | proof_size_bytes | scaling | 5 | 19,133 | 15,769 | -17.6% | [-17.6%, -17.6%] |
| depth 15: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,715,072 | 3,510,272 | -6.3% | [-8.9%, -3.9%] |
| depth 15: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 6,309,501 | 4,710,814 | -24.3% | [-30.5%, -11.7%] |
| depth 15: optimized vs original folded | verify_median_ns | scaling | 5 | 507,405 | 171,086 | -64.0% | [-69.1%, -55.1%] |
| depth 15: optimized vs original folded | proof_size_bytes | scaling | 5 | 20,116 | 15,769 | -21.6% | [-21.6%, -21.6%] |
| depth 15: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 3,846,144 | 3,510,272 | -9.4% | [-10.5%, -7.1%] |
| depth 31: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 12,527,967 | 10,692,746 | -18.8% | [-84.6%, +39.4%] |
| depth 31: optimized vs original baseline | verify_median_ns | scaling | 5 | 301,985 | 257,710 | -17.6% | [-66.0%, +55.3%] |
| depth 31: optimized vs original baseline | proof_size_bytes | scaling | 5 | 20,475 | 19,739 | -3.6% | [-3.6%, -3.6%] |
| depth 31: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 4,276,224 | 3,608,576 | -14.4% | [-16.8%, -13.7%] |
| depth 31: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 12,084,952 | 10,692,746 | -16.1% | [-88.6%, +48.3%] |
| depth 31: optimized vs original folded | verify_median_ns | scaling | 5 | 582,262 | 257,710 | -58.5% | [-128.7%, -10.9%] |
| depth 31: optimized vs original folded | proof_size_bytes | scaling | 5 | 23,931 | 19,739 | -17.5% | [-17.5%, -17.5%] |
| depth 31: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 4,222,976 | 3,608,576 | -14.5% | [-16.0%, -13.3%] |
| depth 63: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 23,310,902 | 19,254,015 | -19.0% | [-28.0%, -11.3%] |
| depth 63: optimized vs original baseline | verify_median_ns | scaling | 5 | 408,371 | 273,323 | -33.5% | [-44.1%, -29.0%] |
| depth 63: optimized vs original baseline | proof_size_bytes | scaling | 5 | 27,389 | 26,113 | -4.7% | [-4.7%, -4.7%] |
| depth 63: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 5,210,112 | 4,386,816 | -15.8% | [-17.0%, -14.4%] |
| depth 63: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 22,489,665 | 19,254,015 | -13.5% | [-19.6%, -9.9%] |
| depth 63: optimized vs original folded | verify_median_ns | scaling | 5 | 575,530 | 273,323 | -52.3% | [-59.2%, -47.2%] |
| depth 63: optimized vs original folded | proof_size_bytes | scaling | 5 | 27,038 | 26,113 | -3.4% | [-3.4%, -3.4%] |
| depth 63: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 5,140,480 | 4,386,816 | -14.7% | [-15.9%, -14.3%] |
