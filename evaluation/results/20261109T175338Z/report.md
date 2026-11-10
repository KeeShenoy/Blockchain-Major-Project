# Benchmark report 20261109T175338Z

Revision 01843c2-dirty, Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz, kernel 6.18.54.1-microsoft-standard-WSL2, rustc 1.97.1 (8bab26f4f 2026-07-14). Depth 7, medians.

| Metric | Original baseline | Original folded | Previous folded | Current optimized | Zero knowledge |
|---|---:|---:|---:|---:|---:|
| Proving, trace + prove, warm (µs) | 2,686.0 | 2,621.4 | 2,571.6 | 2,244.3 | 2,628.8 |
| Proving, prove only, warm (µs) | 1,875.4 | 1,887.7 | 1,867.4 | 1,675.0 | 2,061.3 |
| Trace build, warm (µs) | 810.7 | 733.7 | 706.9 | 569.1 | 567.4 |
| Verification, 30 witnesses (µs) | 214.7 | 382.0 | 357.8 | 122.7 | 158.8 |
| Proof size, 30 witnesses (B) | 15,799 | 17,344 | 17,056 | 12,824 | 19,045 |
| Peak heap during proving (B) | 235,519 | 237,019 | 170,015 | 150,770 | 214,083 |
| Peak RSS after proving (B) | 3,665,920 | 3,665,920 | 3,497,984 | 3,452,928 | 3,801,088 |

## Paired comparisons

An interval containing zero means no established difference.

| Comparison | Metric | Experiment | Pairs | Control | Treatment | Change | 95% CI |
|---|---|---|---:|---:|---:|---:|---|
| zero knowledge vs optimized | prove_end_to_end_ns | main | 15 | 2,244,341 | 2,628,790 | +16.4% | [+15.5%, +22.4%] |
| zero knowledge vs optimized | prove_warm_median_ns | main | 15 | 1,674,966 | 2,061,349 | +22.0% | [+21.0%, +28.0%] |
| zero knowledge vs optimized | prove_first_ns | main | 15 | 2,132,056 | 2,226,652 | +15.4% | [+4.2%, +20.7%] |
| zero knowledge vs optimized | trace_build_warm_median_ns | main | 15 | 569,085 | 567,441 | -0.0% | [-0.7%, +2.4%] |
| zero knowledge vs optimized | verify_median_ns | main | 15 | 122,188 | 166,366 | +31.6% | [+25.8%, +39.5%] |
| zero knowledge vs optimized | peak_rss_after_prove_bytes | main | 15 | 3,452,928 | 3,801,088 | +9.7% | [+8.1%, +10.7%] |
| zero knowledge vs optimized | peak_rss_process_bytes | main | 15 | 4,059,136 | 4,022,272 | -1.3% | [-3.1%, +0.0%] |
| zero knowledge vs optimized | proof_size_bytes | main | 15 | 12,693 | 19,031 | +49.9% | [+47.1%, +55.0%] |
| zero knowledge vs optimized | proof_size_bytes | witness | 30 | 12,824 | 19,045 | +48.2% | [+45.3%, +51.2%] |
| zero knowledge vs optimized | verify_median_ns | witness | 30 | 122,722 | 158,774 | +28.6% | [+27.2%, +30.5%] |
| zero knowledge vs original baseline | prove_end_to_end_ns | main | 15 | 2,686,017 | 2,628,790 | -2.1% | [-4.5%, +9.1%] |
| zero knowledge vs original baseline | prove_warm_median_ns | main | 15 | 1,875,359 | 2,061,349 | +9.9% | [+7.4%, +27.0%] |
| zero knowledge vs original baseline | prove_first_ns | main | 15 | 2,104,005 | 2,226,652 | +5.7% | [+3.3%, +31.2%] |
| zero knowledge vs original baseline | trace_build_warm_median_ns | main | 15 | 810,658 | 567,441 | -29.9% | [-31.2%, -24.4%] |
| zero knowledge vs original baseline | verify_median_ns | main | 15 | 216,699 | 166,366 | -25.2% | [-27.0%, -16.3%] |
| zero knowledge vs original baseline | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,801,088 | +3.4% | [+0.8%, +4.5%] |
| zero knowledge vs original baseline | peak_rss_process_bytes | main | 15 | 4,022,272 | 4,022,272 | -0.6% | [-1.6%, +0.7%] |
| zero knowledge vs original baseline | proof_size_bytes | main | 15 | 15,542 | 19,031 | +22.4% | [+20.2%, +26.6%] |
| zero knowledge vs original baseline | proof_size_bytes | witness | 30 | 15,799 | 19,045 | +19.7% | [+18.0%, +23.6%] |
| zero knowledge vs original baseline | verify_median_ns | witness | 30 | 214,727 | 158,774 | -25.1% | [-29.5%, -23.1%] |
| zero knowledge vs original folded | prove_end_to_end_ns | main | 15 | 2,621,443 | 2,628,790 | +0.3% | [-3.2%, +8.3%] |
| zero knowledge vs original folded | prove_warm_median_ns | main | 15 | 1,887,733 | 2,061,349 | +8.3% | [+5.0%, +17.5%] |
| zero knowledge vs original folded | prove_first_ns | main | 15 | 2,113,886 | 2,226,652 | +9.8% | [+2.0%, +31.5%] |
| zero knowledge vs original folded | trace_build_warm_median_ns | main | 15 | 733,710 | 567,441 | -21.3% | [-24.4%, -17.7%] |
| zero knowledge vs original folded | verify_median_ns | main | 15 | 417,120 | 166,366 | -59.5% | [-62.7%, -58.1%] |
| zero knowledge vs original folded | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,801,088 | +4.4% | [+1.6%, +4.7%] |
| zero knowledge vs original folded | peak_rss_process_bytes | main | 15 | 4,009,984 | 4,022,272 | -0.5% | [-2.1%, +0.7%] |
| zero knowledge vs original folded | proof_size_bytes | main | 15 | 18,194 | 19,031 | +4.6% | [+2.7%, +8.1%] |
| zero knowledge vs original folded | proof_size_bytes | witness | 30 | 17,344 | 19,045 | +10.1% | [+8.7%, +11.5%] |
| zero knowledge vs original folded | verify_median_ns | witness | 30 | 381,962 | 158,774 | -57.2% | [-62.9%, -56.0%] |
| optimized vs original baseline | prove_end_to_end_ns | main | 15 | 2,686,017 | 2,244,341 | -16.6% | [-18.6%, -2.6%] |
| optimized vs original baseline | prove_warm_median_ns | main | 15 | 1,875,359 | 1,674,966 | -10.8% | [-12.8%, +5.1%] |
| optimized vs original baseline | prove_first_ns | main | 15 | 2,104,005 | 2,132,056 | -9.9% | [-15.0%, +24.8%] |
| optimized vs original baseline | trace_build_warm_median_ns | main | 15 | 810,658 | 569,085 | -29.5% | [-32.1%, -20.6%] |
| optimized vs original baseline | verify_median_ns | main | 15 | 216,699 | 122,188 | -44.0% | [-47.3%, -42.5%] |
| optimized vs original baseline | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,452,928 | -6.4% | [-6.8%, -5.4%] |
| optimized vs original baseline | peak_rss_process_bytes | main | 15 | 4,022,272 | 4,059,136 | +1.3% | [+0.3%, +2.7%] |
| optimized vs original baseline | proof_size_bytes | main | 15 | 15,542 | 12,693 | -18.3% | [-18.3%, -18.3%] |
| optimized vs original baseline | proof_size_bytes | witness | 30 | 15,799 | 12,824 | -18.2% | [-19.7%, -16.9%] |
| optimized vs original baseline | verify_median_ns | witness | 30 | 214,727 | 122,722 | -43.9% | [-45.3%, -40.2%] |
| optimized vs original folded | prove_end_to_end_ns | main | 15 | 2,621,443 | 2,244,341 | -14.6% | [-18.1%, -11.8%] |
| optimized vs original folded | prove_warm_median_ns | main | 15 | 1,887,733 | 1,674,966 | -11.3% | [-15.6%, -9.3%] |
| optimized vs original folded | prove_first_ns | main | 15 | 2,113,886 | 2,132,056 | -5.9% | [-12.0%, +25.4%] |
| optimized vs original folded | trace_build_warm_median_ns | main | 15 | 733,710 | 569,085 | -21.3% | [-24.5%, -19.3%] |
| optimized vs original folded | verify_median_ns | main | 15 | 417,120 | 122,188 | -68.5% | [-84.4%, -68.0%] |
| optimized vs original folded | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,452,928 | -5.7% | [-6.4%, -4.8%] |
| optimized vs original folded | peak_rss_process_bytes | main | 15 | 4,009,984 | 4,059,136 | +1.2% | [+0.2%, +2.6%] |
| optimized vs original folded | proof_size_bytes | main | 15 | 18,194 | 12,693 | -30.2% | [-30.2%, -30.2%] |
| optimized vs original folded | proof_size_bytes | witness | 30 | 17,344 | 12,824 | -25.6% | [-27.4%, -23.9%] |
| optimized vs original folded | verify_median_ns | witness | 30 | 381,962 | 122,722 | -67.9% | [-71.8%, -65.5%] |
| optimized vs previous folded | prove_end_to_end_ns | main | 15 | 2,571,576 | 2,244,341 | -12.0% | [-14.3%, -4.5%] |
| optimized vs previous folded | prove_warm_median_ns | main | 15 | 1,867,433 | 1,674,966 | -10.1% | [-12.3%, -1.9%] |
| optimized vs previous folded | prove_first_ns | main | 15 | 2,002,478 | 2,132,056 | -8.0% | [-14.9%, +10.7%] |
| optimized vs previous folded | trace_build_warm_median_ns | main | 15 | 706,856 | 569,085 | -18.2% | [-20.3%, -11.6%] |
| optimized vs previous folded | verify_median_ns | main | 15 | 379,270 | 122,188 | -66.8% | [-74.8%, -65.7%] |
| optimized vs previous folded | peak_rss_after_prove_bytes | main | 15 | 3,497,984 | 3,452,928 | -0.8% | [-1.3%, -0.1%] |
| optimized vs previous folded | peak_rss_process_bytes | main | 15 | 4,018,176 | 4,059,136 | +0.5% | [-0.6%, +3.1%] |
| optimized vs previous folded | proof_size_bytes | main | 15 | 17,393 | 12,693 | -27.0% | [-27.0%, -27.0%] |
| optimized vs previous folded | proof_size_bytes | witness | 30 | 17,056 | 12,824 | -23.6% | [-26.8%, -21.8%] |
| optimized vs previous folded | verify_median_ns | witness | 30 | 357,831 | 122,722 | -64.5% | [-68.8%, -62.0%] |
| optimized vs previous split | prove_end_to_end_ns | main | 15 | 2,623,280 | 2,244,341 | -13.1% | [-15.3%, -8.1%] |
| optimized vs previous split | prove_warm_median_ns | main | 15 | 1,907,518 | 1,674,966 | -10.7% | [-13.7%, -5.0%] |
| optimized vs previous split | prove_first_ns | main | 15 | 2,024,131 | 2,132,056 | -8.6% | [-14.2%, +10.0%] |
| optimized vs previous split | trace_build_warm_median_ns | main | 15 | 708,093 | 569,085 | -19.5% | [-20.0%, -16.3%] |
| optimized vs previous split | verify_median_ns | main | 15 | 223,575 | 122,188 | -44.1% | [-46.3%, -41.8%] |
| optimized vs previous split | peak_rss_after_prove_bytes | main | 15 | 3,469,312 | 3,452,928 | -0.7% | [-1.5%, -0.1%] |
| optimized vs previous split | peak_rss_process_bytes | main | 15 | 4,112,384 | 4,059,136 | -0.2% | [-1.2%, +0.6%] |
| optimized vs previous split | proof_size_bytes | main | 15 | 16,793 | 12,693 | -24.4% | [-24.4%, -24.4%] |
| current folded vs original folded | prove_end_to_end_ns | main | 15 | 2,621,443 | 2,272,849 | -13.4% | [-17.5%, -10.1%] |
| current folded vs original folded | prove_warm_median_ns | main | 15 | 1,887,733 | 1,704,806 | -10.4% | [-15.5%, -6.7%] |
| current folded vs original folded | prove_first_ns | main | 15 | 2,113,886 | 1,785,981 | -8.4% | [-17.5%, -4.3%] |
| current folded vs original folded | trace_build_warm_median_ns | main | 15 | 733,710 | 568,043 | -20.7% | [-23.3%, -17.5%] |
| current folded vs original folded | verify_median_ns | main | 15 | 417,120 | 380,582 | -8.1% | [-11.5%, -5.0%] |
| current folded vs original folded | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,469,312 | -5.0% | [-6.3%, -4.2%] |
| current folded vs original folded | peak_rss_process_bytes | main | 15 | 4,009,984 | 4,071,424 | +1.6% | [+0.2%, +3.5%] |
| current folded vs original folded | proof_size_bytes | main | 15 | 18,194 | 17,393 | -4.4% | [-4.4%, -4.4%] |
| current folded vs original folded | proof_size_bytes | witness | 30 | 17,344 | 17,056 | -1.1% | [-2.8%, +1.3%] |
| current folded vs original folded | verify_median_ns | witness | 30 | 381,962 | 353,264 | -5.4% | [-13.4%, -1.4%] |
| current baseline vs original baseline | prove_end_to_end_ns | main | 15 | 2,686,017 | 2,203,900 | -18.5% | [-19.1%, -16.6%] |
| current baseline vs original baseline | prove_warm_median_ns | main | 15 | 1,875,359 | 1,645,522 | -13.2% | [-14.1%, -11.0%] |
| current baseline vs original baseline | prove_first_ns | main | 15 | 2,104,005 | 1,804,922 | -12.2% | [-13.9%, -7.8%] |
| current baseline vs original baseline | trace_build_warm_median_ns | main | 15 | 810,658 | 559,708 | -30.5% | [-30.9%, -29.3%] |
| current baseline vs original baseline | verify_median_ns | main | 15 | 216,699 | 220,886 | +1.9% | [-3.2%, +4.3%] |
| current baseline vs original baseline | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,559,424 | -2.7% | [-4.0%, -2.5%] |
| current baseline vs original baseline | peak_rss_process_bytes | main | 15 | 4,022,272 | 4,075,520 | +1.9% | [+0.7%, +2.9%] |
| current baseline vs original baseline | proof_size_bytes | main | 15 | 15,542 | 16,793 | +8.0% | [+8.0%, +8.0%] |
| current baseline vs original baseline | proof_size_bytes | witness | 30 | 15,799 | 15,860 | -0.0% | [-3.2%, +2.0%] |
| current baseline vs original baseline | verify_median_ns | witness | 30 | 214,727 | 202,100 | -7.7% | [-9.8%, -2.8%] |
| folded vs baseline (original) | prove_end_to_end_ns | main | 15 | 2,686,017 | 2,621,443 | -3.7% | [-5.3%, -2.4%] |
| folded vs baseline (original) | prove_warm_median_ns | main | 15 | 1,875,359 | 1,887,733 | -0.3% | [-2.7%, +1.8%] |
| folded vs baseline (original) | prove_first_ns | main | 15 | 2,104,005 | 2,113,886 | -2.8% | [-10.2%, +0.9%] |
| folded vs baseline (original) | trace_build_warm_median_ns | main | 15 | 810,658 | 733,710 | -10.5% | [-16.4%, -9.5%] |
| folded vs baseline (original) | verify_median_ns | main | 15 | 216,699 | 417,120 | +88.8% | [+86.3%, +103.2%] |
| folded vs baseline (original) | peak_rss_after_prove_bytes | main | 15 | 3,665,920 | 3,665,920 | -0.2% | [-1.0%, +0.1%] |
| folded vs baseline (original) | peak_rss_process_bytes | main | 15 | 4,022,272 | 4,009,984 | -0.5% | [-1.1%, +0.8%] |
| folded vs baseline (original) | proof_size_bytes | main | 15 | 15,542 | 18,194 | +17.1% | [+17.1%, +17.1%] |
| folded vs baseline (original) | proof_size_bytes | witness | 30 | 15,799 | 17,344 | +9.6% | [+7.1%, +11.6%] |
| folded vs baseline (original) | verify_median_ns | witness | 30 | 214,727 | 381,962 | +79.4% | [+72.9%, +84.5%] |
| folded vs baseline (current) | prove_end_to_end_ns | main | 15 | 2,203,900 | 2,272,849 | +3.4% | [+2.4%, +12.2%] |
| folded vs baseline (current) | prove_warm_median_ns | main | 15 | 1,645,522 | 1,704,806 | +3.8% | [+2.7%, +12.9%] |
| folded vs baseline (current) | prove_first_ns | main | 15 | 1,804,922 | 1,785,981 | -0.1% | [-16.7%, +0.4%] |
| folded vs baseline (current) | trace_build_warm_median_ns | main | 15 | 559,708 | 568,043 | +1.5% | [-0.4%, +6.0%] |
| folded vs baseline (current) | verify_median_ns | main | 15 | 220,886 | 380,582 | +72.7% | [+69.5%, +107.1%] |
| folded vs baseline (current) | peak_rss_after_prove_bytes | main | 15 | 3,559,424 | 3,469,312 | -2.4% | [-3.0%, -2.1%] |
| folded vs baseline (current) | peak_rss_process_bytes | main | 15 | 4,075,520 | 4,071,424 | +0.0% | [-0.7%, +1.3%] |
| folded vs baseline (current) | proof_size_bytes | main | 15 | 16,793 | 17,393 | +3.6% | [+3.6%, +3.6%] |
| folded vs baseline (current) | proof_size_bytes | witness | 30 | 15,860 | 17,056 | +8.7% | [+4.9%, +9.9%] |
| folded vs baseline (current) | verify_median_ns | witness | 30 | 202,100 | 353,264 | +74.8% | [+69.7%, +82.3%] |
| optimized vs folded (current) | prove_end_to_end_ns | main | 15 | 2,272,849 | 2,244,341 | -1.3% | [-5.1%, +0.9%] |
| optimized vs folded (current) | prove_warm_median_ns | main | 15 | 1,704,806 | 1,674,966 | -1.5% | [-5.1%, +0.1%] |
| optimized vs folded (current) | prove_first_ns | main | 15 | 1,785,981 | 2,132,056 | +2.7% | [-1.9%, +24.1%] |
| optimized vs folded (current) | trace_build_warm_median_ns | main | 15 | 568,043 | 569,085 | -1.0% | [-4.2%, +3.2%] |
| optimized vs folded (current) | verify_median_ns | main | 15 | 380,582 | 122,188 | -69.1% | [-95.4%, -65.9%] |
| optimized vs folded (current) | peak_rss_after_prove_bytes | main | 15 | 3,469,312 | 3,452,928 | -0.6% | [-1.5%, +0.1%] |
| optimized vs folded (current) | peak_rss_process_bytes | main | 15 | 4,071,424 | 4,059,136 | -0.4% | [-2.2%, +0.7%] |
| optimized vs folded (current) | proof_size_bytes | main | 15 | 17,393 | 12,693 | -27.0% | [-27.0%, -27.0%] |
| optimized vs folded (current) | proof_size_bytes | witness | 30 | 17,056 | 12,824 | -23.6% | [-26.8%, -21.8%] |
| optimized vs folded (current) | verify_median_ns | witness | 30 | 353,264 | 122,722 | -65.9% | [-69.3%, -62.6%] |
| optimized vs baseline (current) | prove_end_to_end_ns | main | 15 | 2,203,900 | 2,244,341 | +2.3% | [-0.6%, +5.6%] |
| optimized vs baseline (current) | prove_warm_median_ns | main | 15 | 1,645,522 | 1,674,966 | +2.5% | [-0.8%, +6.4%] |
| optimized vs baseline (current) | prove_first_ns | main | 15 | 1,804,922 | 2,132,056 | +2.7% | [-2.0%, +42.9%] |
| optimized vs baseline (current) | trace_build_warm_median_ns | main | 15 | 559,708 | 569,085 | +1.3% | [-2.5%, +3.7%] |
| optimized vs baseline (current) | verify_median_ns | main | 15 | 220,886 | 122,188 | -44.1% | [-46.7%, -43.3%] |
| optimized vs baseline (current) | peak_rss_after_prove_bytes | main | 15 | 3,559,424 | 3,452,928 | -2.8% | [-4.0%, -2.2%] |
| optimized vs baseline (current) | peak_rss_process_bytes | main | 15 | 4,075,520 | 4,059,136 | +0.0% | [-0.4%, +0.7%] |
| optimized vs baseline (current) | proof_size_bytes | main | 15 | 16,793 | 12,693 | -24.4% | [-24.4%, -24.4%] |
| optimized vs baseline (current) | proof_size_bytes | witness | 30 | 15,860 | 12,824 | -18.0% | [-19.0%, -16.3%] |
| optimized vs baseline (current) | verify_median_ns | witness | 30 | 202,100 | 122,722 | -38.3% | [-40.8%, -36.6%] |
| storage: compact vs Winterfell, FRI 8 | prove_end_to_end_ns | main | 15 | 2,203,900 | 2,241,345 | +1.8% | [-0.4%, +2.1%] |
| storage: compact vs Winterfell, FRI 8 | prove_warm_median_ns | main | 15 | 1,645,522 | 1,688,694 | +2.5% | [+0.2%, +2.8%] |
| storage: compact vs Winterfell, FRI 8 | prove_first_ns | main | 15 | 1,804,922 | 1,844,298 | +0.0% | [-16.8%, +1.8%] |
| storage: compact vs Winterfell, FRI 8 | trace_build_warm_median_ns | main | 15 | 559,708 | 561,545 | +0.0% | [-1.9%, +1.7%] |
| storage: compact vs Winterfell, FRI 8 | verify_median_ns | main | 15 | 220,886 | 217,793 | -0.1% | [-1.9%, +0.5%] |
| storage: compact vs Winterfell, FRI 8 | peak_rss_after_prove_bytes | main | 15 | 3,559,424 | 3,461,120 | -3.0% | [-3.6%, -2.5%] |
| storage: compact vs Winterfell, FRI 8 | peak_rss_process_bytes | main | 15 | 4,075,520 | 4,079,616 | +0.0% | [-0.8%, +2.2%] |
| storage: compact vs Winterfell, FRI 8 | proof_size_bytes | main | 15 | 16,793 | 16,793 | +0.0% | [+0.0%, +0.0%] |
| storage: compact vs Winterfell, FRI 8 | proof_size_bytes | witness | 30 | 15,860 | 15,860 | +0.0% | [+0.0%, +0.0%] |
| storage: compact vs Winterfell, FRI 8 | verify_median_ns | witness | 30 | 202,100 | 202,294 | -0.2% | [-0.7%, +0.3%] |
| FRI shape: optimized vs split | prove_end_to_end_ns | main | 15 | 2,241,345 | 2,244,341 | +0.3% | [-0.4%, +7.1%] |
| FRI shape: optimized vs split | prove_warm_median_ns | main | 15 | 1,688,694 | 1,674,966 | +0.1% | [-0.9%, +9.3%] |
| FRI shape: optimized vs split | prove_first_ns | main | 15 | 1,844,298 | 2,132,056 | +14.5% | [-6.2%, +38.9%] |
| FRI shape: optimized vs split | trace_build_warm_median_ns | main | 15 | 561,545 | 569,085 | +1.3% | [+0.5%, +5.3%] |
| FRI shape: optimized vs split | verify_median_ns | main | 15 | 217,793 | 122,188 | -44.4% | [-69.1%, -42.3%] |
| FRI shape: optimized vs split | peak_rss_after_prove_bytes | main | 15 | 3,461,120 | 3,452,928 | -0.6% | [-1.1%, +0.5%] |
| FRI shape: optimized vs split | peak_rss_process_bytes | main | 15 | 4,079,616 | 4,059,136 | -0.4% | [-1.6%, +0.8%] |
| FRI shape: optimized vs split | proof_size_bytes | main | 15 | 16,793 | 12,693 | -24.4% | [-24.4%, -24.4%] |
| FRI shape: optimized vs split | proof_size_bytes | witness | 30 | 15,860 | 12,824 | -18.0% | [-19.0%, -16.3%] |
| FRI shape: optimized vs split | verify_median_ns | witness | 30 | 202,294 | 122,722 | -38.2% | [-39.8%, -36.6%] |
| Rescue arithmetic: exact routines vs field operators | prove_end_to_end_ns | main | 15 | 2,595,150 | 2,244,341 | -13.3% | [-15.8%, -11.7%] |
| Rescue arithmetic: exact routines vs field operators | prove_warm_median_ns | main | 15 | 1,906,583 | 1,674,966 | -10.6% | [-14.5%, -8.8%] |
| Rescue arithmetic: exact routines vs field operators | prove_first_ns | main | 15 | 2,135,930 | 2,132,056 | -6.4% | [-10.5%, +2.5%] |
| Rescue arithmetic: exact routines vs field operators | trace_build_warm_median_ns | main | 15 | 698,233 | 569,085 | -19.3% | [-21.9%, -18.5%] |
| Rescue arithmetic: exact routines vs field operators | verify_median_ns | main | 15 | 122,962 | 122,188 | +0.0% | [-0.9%, +2.8%] |
| Rescue arithmetic: exact routines vs field operators | peak_rss_after_prove_bytes | main | 15 | 3,416,064 | 3,452,928 | +0.6% | [-0.5%, +2.5%] |
| Rescue arithmetic: exact routines vs field operators | peak_rss_process_bytes | main | 15 | 4,009,984 | 4,059,136 | +1.1% | [+0.4%, +1.6%] |
| Rescue arithmetic: exact routines vs field operators | proof_size_bytes | main | 15 | 12,693 | 12,693 | +0.0% | [+0.0%, +0.0%] |
| trade-off: Blake3-192 vs Blake3-256 | proof_size_bytes | witness | 30 | 12,824 | 11,440 | -10.4% | [-14.3%, -9.2%] |
| trade-off: Blake3-192 vs Blake3-256 | verify_median_ns | witness | 30 | 122,722 | 120,462 | -2.8% | [-5.3%, -0.7%] |
| build: x86-64-v3 vs portable | prove_end_to_end_ns | main | 15 | 2,244,341 | 2,156,040 | -5.1% | [-9.5%, -2.1%] |
| build: x86-64-v3 vs portable | prove_warm_median_ns | main | 15 | 1,674,966 | 1,625,429 | -4.1% | [-7.4%, -1.4%] |
| build: x86-64-v3 vs portable | prove_first_ns | main | 15 | 2,132,056 | 1,867,866 | -4.4% | [-29.4%, -1.8%] |
| build: x86-64-v3 vs portable | trace_build_warm_median_ns | main | 15 | 569,085 | 530,611 | -8.1% | [-15.6%, -4.3%] |
| build: x86-64-v3 vs portable | verify_median_ns | main | 15 | 122,188 | 118,155 | -2.6% | [-5.7%, +3.6%] |
| build: x86-64-v3 vs portable | peak_rss_after_prove_bytes | main | 15 | 3,452,928 | 3,477,504 | +0.6% | [-0.7%, +1.7%] |
| build: x86-64-v3 vs portable | peak_rss_process_bytes | main | 15 | 4,059,136 | 4,075,520 | +0.4% | [-1.1%, +0.7%] |
| build: x86-64-v3 vs portable | proof_size_bytes | main | 15 | 12,693 | 12,693 | +0.0% | [+0.0%, +0.0%] |
| build: static vs dynamic | prove_end_to_end_ns | main | 15 | 2,244,341 | 2,255,027 | +0.8% | [-4.6%, +4.6%] |
| build: static vs dynamic | prove_warm_median_ns | main | 15 | 1,674,966 | 1,679,828 | +1.0% | [-5.0%, +9.2%] |
| build: static vs dynamic | prove_first_ns | main | 15 | 2,132,056 | 1,829,539 | -4.3% | [-28.3%, -0.6%] |
| build: static vs dynamic | trace_build_warm_median_ns | main | 15 | 569,085 | 575,199 | -0.1% | [-10.1%, +3.2%] |
| build: static vs dynamic | verify_median_ns | main | 15 | 122,188 | 120,486 | -0.5% | [-3.8%, +4.4%] |
| build: static vs dynamic | peak_rss_after_prove_bytes | main | 15 | 3,452,928 | 2,211,840 | -36.1% | [-36.3%, -35.2%] |
| build: static vs dynamic | peak_rss_process_bytes | main | 15 | 4,059,136 | 2,564,096 | -36.8% | [-37.9%, -36.5%] |
| build: static vs dynamic | proof_size_bytes | main | 15 | 12,693 | 12,693 | +0.0% | [+0.0%, +0.0%] |
| heap: zero knowledge vs optimized | heap_peak_prove_bytes | heap | 3 | 150,770 | 214,083 | +42.0% | [+41.6%, +43.4%] |
| heap: zero knowledge vs original baseline | heap_peak_prove_bytes | heap | 3 | 235,519 | 214,083 | -9.1% | [-9.3%, -8.2%] |
| heap: optimized vs original baseline | heap_peak_prove_bytes | heap | 3 | 235,519 | 150,770 | -36.0% | [-36.0%, -36.0%] |
| heap: optimized vs original folded | heap_peak_prove_bytes | heap | 3 | 237,019 | 150,770 | -36.4% | [-36.4%, -36.4%] |
| heap: optimized vs previous compact | heap_peak_prove_bytes | heap | 3 | 170,015 | 150,770 | -11.3% | [-11.3%, -11.3%] |
| heap: folded vs folded with Winterfell storage | heap_peak_prove_bytes | heap | 3 | 236,543 | 156,135 | -34.0% | [-34.0%, -34.0%] |
| depth 3: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 1,398,432 | 1,155,060 | -17.2% | [-18.7%, -14.3%] |
| depth 3: optimized vs original baseline | verify_median_ns | scaling | 5 | 108,756 | 99,652 | -8.5% | [-12.7%, -7.2%] |
| depth 3: optimized vs original baseline | proof_size_bytes | scaling | 5 | 11,225 | 10,323 | -8.0% | [-8.0%, -8.0%] |
| depth 3: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,555,328 | 3,358,720 | -5.5% | [-5.6%, -2.4%] |
| depth 3: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 1,315,340 | 1,155,060 | -12.8% | [-13.6%, -8.4%] |
| depth 3: optimized vs original folded | verify_median_ns | scaling | 5 | 108,950 | 99,652 | -9.5% | [-10.4%, -7.8%] |
| depth 3: optimized vs original folded | proof_size_bytes | scaling | 5 | 11,291 | 10,323 | -8.6% | [-8.6%, -8.6%] |
| depth 3: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 3,559,424 | 3,358,720 | -5.3% | [-5.9%, -2.8%] |
| depth 7: zero knowledge vs optimized | prove_end_to_end_ns | scaling | 5 | 2,242,041 | 2,633,195 | +16.5% | [+14.6%, +18.9%] |
| depth 7: zero knowledge vs optimized | verify_median_ns | scaling | 5 | 121,443 | 160,568 | +30.8% | [+27.0%, +40.4%] |
| depth 7: zero knowledge vs optimized | proof_size_bytes | scaling | 5 | 12,693 | 19,141 | +50.8% | [+44.1%, +57.5%] |
| depth 7: zero knowledge vs optimized | peak_rss_after_prove_bytes | scaling | 5 | 3,399,680 | 3,788,800 | +10.8% | [+5.5%, +12.7%] |
| depth 7: zero knowledge vs original baseline | prove_end_to_end_ns | scaling | 5 | 2,710,865 | 2,633,195 | -4.5% | [-6.5%, -1.3%] |
| depth 7: zero knowledge vs original baseline | verify_median_ns | scaling | 5 | 216,350 | 160,568 | -25.6% | [-29.7%, -23.7%] |
| depth 7: zero knowledge vs original baseline | proof_size_bytes | scaling | 5 | 15,542 | 19,141 | +23.2% | [+17.7%, +28.6%] |
| depth 7: zero knowledge vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,665,920 | 3,788,800 | +2.3% | [+0.4%, +4.4%] |
| depth 7: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 2,710,865 | 2,242,041 | -17.7% | [-18.8%, -16.7%] |
| depth 7: optimized vs original baseline | verify_median_ns | scaling | 5 | 216,350 | 121,443 | -43.5% | [-48.3%, -41.0%] |
| depth 7: optimized vs original baseline | proof_size_bytes | scaling | 5 | 15,542 | 12,693 | -18.3% | [-18.3%, -18.3%] |
| depth 7: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,665,920 | 3,399,680 | -7.4% | [-8.3%, -4.7%] |
| depth 7: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 2,566,469 | 2,242,041 | -13.0% | [-14.8%, -12.6%] |
| depth 7: optimized vs original folded | verify_median_ns | scaling | 5 | 406,552 | 121,443 | -70.0% | [-73.0%, -68.4%] |
| depth 7: optimized vs original folded | proof_size_bytes | scaling | 5 | 18,194 | 12,693 | -30.2% | [-30.2%, -30.2%] |
| depth 7: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 3,661,824 | 3,399,680 | -6.4% | [-8.4%, -4.6%] |
| depth 15: zero knowledge vs optimized | prove_end_to_end_ns | scaling | 5 | 4,412,322 | 4,817,130 | +9.2% | [+8.7%, +11.7%] |
| depth 15: zero knowledge vs optimized | verify_median_ns | scaling | 5 | 158,728 | 170,834 | +7.4% | [+4.1%, +14.6%] |
| depth 15: zero knowledge vs optimized | proof_size_bytes | scaling | 5 | 15,769 | 19,805 | +25.6% | [+20.3%, +28.4%] |
| depth 15: zero knowledge vs optimized | peak_rss_after_prove_bytes | scaling | 5 | 3,604,480 | 3,817,472 | +5.1% | [+3.9%, +7.7%] |
| depth 15: zero knowledge vs original baseline | prove_end_to_end_ns | scaling | 5 | 5,338,154 | 4,817,130 | -9.1% | [-10.7%, -7.6%] |
| depth 15: zero knowledge vs original baseline | verify_median_ns | scaling | 5 | 242,305 | 170,834 | -29.5% | [-29.7%, -25.5%] |
| depth 15: zero knowledge vs original baseline | proof_size_bytes | scaling | 5 | 19,133 | 19,805 | +3.5% | [-0.8%, +5.9%] |
| depth 15: zero knowledge vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,706,880 | 3,817,472 | +2.2% | [+0.9%, +5.1%] |
| depth 15: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 5,338,154 | 4,412,322 | -17.2% | [-17.8%, -16.7%] |
| depth 15: optimized vs original baseline | verify_median_ns | scaling | 5 | 242,305 | 158,728 | -34.3% | [-35.1%, -32.3%] |
| depth 15: optimized vs original baseline | proof_size_bytes | scaling | 5 | 19,133 | 15,769 | -17.6% | [-17.6%, -17.6%] |
| depth 15: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 3,706,880 | 3,604,480 | -2.8% | [-6.3%, +1.3%] |
| depth 15: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 5,216,323 | 4,412,322 | -15.0% | [-16.7%, -14.2%] |
| depth 15: optimized vs original folded | verify_median_ns | scaling | 5 | 426,793 | 158,728 | -62.6% | [-63.7%, -59.5%] |
| depth 15: optimized vs original folded | proof_size_bytes | scaling | 5 | 20,116 | 15,769 | -21.6% | [-21.6%, -21.6%] |
| depth 15: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 3,837,952 | 3,604,480 | -6.1% | [-7.2%, -3.2%] |
| depth 31: zero knowledge vs optimized | prove_end_to_end_ns | scaling | 5 | 8,859,518 | 9,342,151 | +5.7% | [-4.1%, +8.9%] |
| depth 31: zero knowledge vs optimized | verify_median_ns | scaling | 5 | 215,130 | 219,431 | +2.0% | [-9.1%, +4.2%] |
| depth 31: zero knowledge vs optimized | proof_size_bytes | scaling | 5 | 19,739 | 22,283 | +12.9% | [+11.3%, +14.8%] |
| depth 31: zero knowledge vs optimized | peak_rss_after_prove_bytes | scaling | 5 | 3,682,304 | 4,198,400 | +12.5% | [+9.1%, +17.7%] |
| depth 31: zero knowledge vs original baseline | prove_end_to_end_ns | scaling | 5 | 10,975,457 | 9,342,151 | -12.9% | [-16.5%, -11.3%] |
| depth 31: zero knowledge vs original baseline | verify_median_ns | scaling | 5 | 267,790 | 219,431 | -17.4% | [-25.1%, -15.5%] |
| depth 31: zero knowledge vs original baseline | proof_size_bytes | scaling | 5 | 20,475 | 22,283 | +8.8% | [+7.3%, +10.7%] |
| depth 31: zero knowledge vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 4,280,320 | 4,198,400 | -2.0% | [-4.9%, +2.1%] |
| depth 31: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 10,975,457 | 8,859,518 | -17.5% | [-22.8%, -11.1%] |
| depth 31: optimized vs original baseline | verify_median_ns | scaling | 5 | 267,790 | 215,130 | -18.8% | [-22.2%, -14.7%] |
| depth 31: optimized vs original baseline | proof_size_bytes | scaling | 5 | 20,475 | 19,739 | -3.6% | [-3.6%, -3.6%] |
| depth 31: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 4,280,320 | 3,682,304 | -13.1% | [-15.3%, -12.5%] |
| depth 31: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 10,845,290 | 8,859,518 | -17.4% | [-24.8%, -10.6%] |
| depth 31: optimized vs original folded | verify_median_ns | scaling | 5 | 514,362 | 215,130 | -58.2% | [-151.6%, -50.7%] |
| depth 31: optimized vs original folded | proof_size_bytes | scaling | 5 | 23,931 | 19,739 | -17.5% | [-17.5%, -17.5%] |
| depth 31: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 4,284,416 | 3,682,304 | -14.0% | [-15.5%, -12.4%] |
| depth 63: zero knowledge vs optimized | prove_end_to_end_ns | scaling | 5 | 18,649,546 | 18,973,213 | +3.5% | [+0.7%, +3.7%] |
| depth 63: zero knowledge vs optimized | verify_median_ns | scaling | 5 | 255,472 | 322,934 | +27.1% | [+24.9%, +29.7%] |
| depth 63: zero knowledge vs optimized | proof_size_bytes | scaling | 5 | 26,113 | 27,655 | +5.9% | [+4.9%, +9.1%] |
| depth 63: zero knowledge vs optimized | peak_rss_after_prove_bytes | scaling | 5 | 4,472,832 | 4,857,856 | +8.6% | [+4.6%, +13.4%] |
| depth 63: zero knowledge vs original baseline | prove_end_to_end_ns | scaling | 5 | 21,368,814 | 18,973,213 | -11.0% | [-12.7%, -10.2%] |
| depth 63: zero knowledge vs original baseline | verify_median_ns | scaling | 5 | 383,787 | 322,934 | -16.8% | [-17.9%, -13.1%] |
| depth 63: zero knowledge vs original baseline | proof_size_bytes | scaling | 5 | 27,389 | 27,655 | +1.0% | [+0.0%, +4.0%] |
| depth 63: zero knowledge vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 5,173,248 | 4,857,856 | -6.1% | [-6.5%, -4.8%] |
| depth 63: optimized vs original baseline | prove_end_to_end_ns | scaling | 5 | 21,368,814 | 18,649,546 | -13.5% | [-15.8%, -11.6%] |
| depth 63: optimized vs original baseline | verify_median_ns | scaling | 5 | 383,787 | 255,472 | -33.3% | [-35.3%, -32.5%] |
| depth 63: optimized vs original baseline | proof_size_bytes | scaling | 5 | 27,389 | 26,113 | -4.7% | [-4.7%, -4.7%] |
| depth 63: optimized vs original baseline | peak_rss_after_prove_bytes | scaling | 5 | 5,173,248 | 4,472,832 | -13.5% | [-16.4%, -10.5%] |
| depth 63: optimized vs original folded | prove_end_to_end_ns | scaling | 5 | 21,419,735 | 18,649,546 | -13.7% | [-14.6%, -12.4%] |
| depth 63: optimized vs original folded | verify_median_ns | scaling | 5 | 538,674 | 255,472 | -52.5% | [-54.6%, -51.6%] |
| depth 63: optimized vs original folded | proof_size_bytes | scaling | 5 | 27,038 | 26,113 | -3.4% | [-3.4%, -3.4%] |
| depth 63: optimized vs original folded | peak_rss_after_prove_bytes | scaling | 5 | 5,181,440 | 4,472,832 | -13.8% | [-16.2%, -11.8%] |
