#!/usr/bin/env python3
"""Draws charts as SVG from a benchmark result directory. Standard library only.

run_benchmarks.py calls this automatically. It can also be run alone on any earlier result:

    python3 evaluation/plot_graphs.py evaluation/results/<timestamp>

All values come from <directory>/summary.json and stages.json. Output: <directory>/charts/*.svg.
Bars and points show medians; whiskers span the observed minimum and maximum.
"""

import json
import math
import pathlib
import sys
from xml.sax.saxutils import escape

TEXT, MUTED, GRID, SURFACE = "#0b0b0b", "#52514e", "#e4e3df", "#fcfcfb"
FONT = "font-family='system-ui, -apple-system, Segoe UI, sans-serif'"
PALETTE = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4"]

ORIGINAL_BASELINE = ("original", "baseline")
ORIGINAL_FOLDED = ("original", "folded")
PREVIOUS_FOLDED = ("previous", "folded")
CURRENT_BASELINE = ("current", "baseline")
CURRENT_OPTIMIZED = ("current", "optimized")

# Color follows the configuration, so every chart in a run reads the same way.
LINE_SERIES = [
    (ORIGINAL_BASELINE, "Original baseline (FRI 8)", PALETTE[0]),
    (ORIGINAL_FOLDED, "Original folded (FRI 16)", PALETTE[1]),
    (PREVIOUS_FOLDED, "Previous compact (FRI 16)", PALETTE[2]),
    (CURRENT_BASELINE, "Current baseline (FRI 8)", PALETTE[3]),
    (CURRENT_OPTIMIZED, "Current optimized", PALETTE[4]),
]
BAR_CONFIGS = [
    (("original", "dynamic", "baseline"), "Original\nbaseline", PALETTE[0]),
    (("original", "dynamic", "folded"), "Original\nfolded", PALETTE[1]),
    (("previous", "dynamic", "folded"), "Previous\ncompact", PALETTE[2]),
    (("current", "dynamic", "baseline"), "Current\nbaseline", PALETTE[3]),
    (("current", "dynamic", "folded"), "Current\nfolded", PALETTE[3]),
    (("current", "dynamic", "split"), "Current\nsplit", PALETTE[3]),
    (("current", "dynamic", "optimized"), "Current\noptimized", PALETTE[4]),
]
BUILD_CONFIGS = [
    (("current", "generic-arith", "optimized"), "Field-operator\nRescue", PALETTE[3]),
    (("current", "dynamic", "optimized"), "Default\nbuild", PALETTE[4]),
    (("current", "v3", "optimized"), "x86-64-v3", PALETTE[2]),
    (("current", "static", "optimized"), "Static", PALETTE[0]),
]

BAR_CHARTS = [
    ("compare_prove_time", "Trace build + proving, warm", "main", "prove_end_to_end_ns", "µs", 1e3,
     BAR_CONFIGS),
    ("compare_prove_only", "Proving only (no trace build), warm", "main", "prove_warm_median_ns",
     "µs", 1e3, BAR_CONFIGS),
    ("compare_verify_time", "Verification per proof (30 witnesses)", "witness", "verify_median_ns",
     "µs", 1e3, BAR_CONFIGS),
    ("compare_proof_size", "Serialized proof size (30 witnesses)", "witness", "proof_size_bytes",
     "KiB", 1024, BAR_CONFIGS),
    ("compare_heap", "Peak proving heap (allocator-tracked)", "heap", "heap_peak_prove_bytes",
     "KiB", 1024, None),
    ("compare_rss", "Peak RSS after proving (VmHWM)", "main", "peak_rss_after_prove_bytes", "MiB",
     1024 * 1024, BAR_CONFIGS),
    ("builds_prove_time", "Optimized mode: effect of arithmetic and build, trace + prove", "main",
     "prove_end_to_end_ns", "µs", 1e3, BUILD_CONFIGS),
    ("builds_rss", "Optimized mode: effect of arithmetic and build, peak RSS", "main",
     "peak_rss_after_prove_bytes", "MiB", 1024 * 1024, BUILD_CONFIGS),
]
LINE_CHARTS = [
    ("scaling_prove_time", "Trace build + proving vs tree depth", "scaling",
     "prove_end_to_end_ns", "ms", 1e6),
    ("scaling_verify_time", "Verification vs tree depth", "scaling", "verify_median_ns", "µs", 1e3),
    ("scaling_proof_size", "Proof size vs tree depth", "scaling", "proof_size_bytes", "KiB", 1024),
    ("scaling_heap", "Peak proving heap vs tree depth", "scaling-heap", "heap_peak_prove_bytes",
     "KiB", 1024),
    ("scaling_rss", "Peak RSS after proving vs tree depth", "scaling",
     "peak_rss_after_prove_bytes", "MiB", 1024 * 1024),
]


def nice_max(value):
    """Smallest round number at or above `value`, so that quarter ticks stay readable."""
    magnitude = 10 ** math.floor(math.log10(value)) if value > 0 else 1
    for step in (1, 1.2, 1.6, 2, 2.4, 3, 4, 5, 6, 8, 10):
        if value <= step * magnitude:
            return step * magnitude
    return 10 * magnitude


def label_number(value):
    return f"{value:,.0f}" if value >= 100 else f"{value:.3g}"


def svg(width, height, body):
    return (f"<svg xmlns='http://www.w3.org/2000/svg' width='{width}' height='{height}' "
            f"viewBox='0 0 {width} {height}' {FONT}>"
            f"<rect width='{width}' height='{height}' fill='{SURFACE}'/>{body}</svg>\n")


def title_text(x, text):
    return (f"<text x='{x}' y='26' font-size='15' font-weight='600' fill='{TEXT}'>"
            f"{escape(text)}</text>")


def y_axis(left, top, plot_height, plot_width, maximum, unit):
    parts = []
    for tick in range(5):
        value = maximum * tick / 4
        y = top + plot_height - plot_height * tick / 4
        parts.append(f"<line x1='{left}' x2='{left + plot_width}' y1='{y}' y2='{y}' "
                     f"stroke='{GRID}' stroke-width='1'/>"
                     f"<text x='{left - 8}' y='{y + 4}' font-size='11' fill='{MUTED}' "
                     f"text-anchor='end'>{value:g}</text>")
    parts.append(f"<text x='{left - 8}' y='{top - 12}' font-size='11' fill='{MUTED}' "
                 f"text-anchor='end'>{escape(unit)}</text>")
    return "".join(parts)


def find(summary, experiment, config, metric, depth=7):
    implementation, build, mode = config
    for entry in summary:
        if (entry["experiment"], entry["implementation"], entry["build"], entry["mode"],
                entry["tree_depth"]) == (experiment, implementation, build, mode, depth) \
                and metric in entry:
            return entry[metric]
    return None


def bar_chart(summary, title, experiment, metric, unit, divisor, configs):
    if configs is None:
        configs = [((implementation, "alloc-profile", mode), label, color)
                   for ((implementation, _, mode), label, color) in BAR_CONFIGS]
    values = []
    for config, label, color in configs:
        stats = find(summary, experiment, config, metric)
        if stats:
            values.append((label, color, {key: stats[key] / divisor
                                          for key in ("median", "min", "max")}))
    if not values:
        return None
    width, height, left, top, plot_height = 640, 350, 64, 62, 220
    plot_width = width - left - 24
    maximum = nice_max(max(value["max"] for _, _, value in values) * 1.12)
    scale = plot_height / maximum
    body = [title_text(left - 48, title), y_axis(left, top, plot_height, plot_width, maximum, unit)]
    slot = plot_width / len(values)
    bar_width = min(slot - 14, 56)
    for index, (label, color, value) in enumerate(values):
        center = left + slot * (index + 0.5)
        bar_height = value["median"] * scale
        x, y = center - bar_width / 2, top + plot_height - bar_height
        body.append(f"<path d='M{x},{top + plot_height} v{-(bar_height - 4)} q0,-4 4,-4 "
                    f"h{bar_width - 8} q4,0 4,4 v{bar_height - 4} z' fill='{color}'>"
                    f"<title>{escape(label.replace(chr(10), ' '))}: {value['median']:.3f} "
                    f"{escape(unit)}</title></path>")
        if value["max"] > value["min"]:
            body.append(f"<line x1='{center}' x2='{center}' "
                        f"y1='{top + plot_height - value['max'] * scale}' "
                        f"y2='{top + plot_height - value['min'] * scale}' stroke='{TEXT}' "
                        f"stroke-width='1' opacity='0.5'/>")
        label_y = top + plot_height - value["max"] * scale - 6
        body.append(f"<text x='{center}' y='{label_y}' font-size='11' fill='{TEXT}' "
                    f"text-anchor='middle'>{label_number(value['median'])}</text>")
        for line, part in enumerate(label.split("\n")):
            body.append(f"<text x='{center}' y='{top + plot_height + 18 + line * 14}' "
                        f"font-size='11' fill='{MUTED}' text-anchor='middle'>{escape(part)}</text>")
    body.append(f"<line x1='{left}' x2='{left + plot_width}' y1='{top + plot_height}' "
                f"y2='{top + plot_height}' stroke='{MUTED}' stroke-width='1'/>")
    return svg(width, height, "".join(body))


def legend_rows(x, y, series, columns=2):
    parts = []
    for index, (label, color) in enumerate(series):
        left = x + (index % columns) * 240
        row_y = y + (index // columns) * 16
        parts.append(f"<rect x='{left}' y='{row_y - 9}' width='10' height='10' rx='2' "
                     f"fill='{color}'/><text x='{left + 16}' y='{row_y}' font-size='11' "
                     f"fill='{MUTED}'>{escape(label)}</text>")
    return "".join(parts)


def line_chart(title, x_values, x_label, series, unit):
    """`series` is a list of (label, color, {x: y}). One y axis; no secondary scale."""
    points = [value for _, _, data in series for value in data.values()]
    if not points or len(x_values) < 2:
        return None
    # The legend takes up to three rows above the plot.
    width, height, left, top, plot_height = 640, 400, 64, 118, 210
    plot_width = width - left - 64
    maximum = nice_max(max(points) * 1.08)
    x_of = {x: left + plot_width * index / (len(x_values) - 1) for index, x in enumerate(x_values)}
    y_of = lambda value: top + plot_height - plot_height * value / maximum  # noqa: E731
    body = [title_text(left - 48, title),
            legend_rows(left - 48, 50, [(label, color) for label, color, _ in series]),
            y_axis(left, top, plot_height, plot_width, maximum, unit)]
    for x, position in x_of.items():
        body.append(f"<text x='{position}' y='{top + plot_height + 20}' font-size='12' "
                    f"fill='{MUTED}' text-anchor='middle'>{x}</text>")
    body.append(f"<text x='{left + plot_width / 2}' y='{top + plot_height + 42}' font-size='12' "
                f"fill='{MUTED}' text-anchor='middle'>{escape(x_label)}</text>")
    last_labels = []
    for label, color, data in series:
        ordered = [(x, data[x]) for x in x_values if x in data]
        path = " ".join(f"{'M' if i == 0 else 'L'}{x_of[x]},{y_of(v)}"
                        for i, (x, v) in enumerate(ordered))
        body.append(f"<path d='{path}' fill='none' stroke='{color}' stroke-width='2'/>")
        for x, value in ordered:
            body.append(f"<circle cx='{x_of[x]}' cy='{y_of(value)}' r='4' fill='{color}' "
                        f"stroke='{SURFACE}' stroke-width='2'><title>{escape(label)}, {x}: "
                        f"{value:.3f} {escape(unit)}</title></circle>")
        if ordered:
            last_labels.append((y_of(ordered[-1][1]), ordered[-1]))
    # End labels, spread apart so that near-equal series stay readable.
    last_labels.sort(key=lambda item: item[0])
    previous_y = -100
    for y, (x, value) in last_labels:
        y = max(y + 4, previous_y + 12)
        previous_y = y
        body.append(f"<text x='{x_of[x] + 8}' y='{y}' font-size='11' fill='{TEXT}'>"
                    f"{label_number(value)}</text>")
    return svg(width, height, "".join(body))


def scaling_chart(summary, title, experiment, metric, unit, divisor):
    depths = sorted({entry["tree_depth"] for entry in summary if entry["experiment"] == experiment})
    build = "alloc-profile" if experiment.endswith("heap") else "dynamic"
    series = []
    for (implementation, mode), label, color in LINE_SERIES:
        data = {}
        for depth in depths:
            stats = find(summary, experiment, (implementation, build, mode), metric, depth)
            if stats:
                data[depth] = stats["median"] / divisor
        if data:
            series.append((label, color, data))
    return line_chart(title, depths, "tree depth (2^depth credentials)", series, unit)


def sweep_chart(summary, title, metric, unit, divisor, depth):
    remainders = sorted({entry["fri_remainder_max_degree"] for entry in summary
                         if entry["experiment"] == "sweep"})
    series = []
    for index, folding in enumerate((2, 4, 8, 16)):
        data = {}
        for entry in summary:
            if entry["experiment"] == "sweep" and entry["tree_depth"] == depth \
                    and entry["fri_folding_factor"] == folding and metric in entry:
                data[entry["fri_remainder_max_degree"]] = entry[metric]["median"] / divisor
        if data:
            series.append((f"FRI folding factor {folding}", PALETTE[index], data))
    return line_chart(f"{title} (depth {depth})", remainders, "FRI remainder max degree", series,
                      unit)


def stage_chart(stages, mode, depth):
    entry = next((item for item in stages
                  if item["mode"] == mode and item["tree_depth"] == depth), None)
    if not entry or not entry["prove_stages_mean_ns"]:
        return None
    names = {
        "evaluate_constraints": "Constraint evaluation",
        "commit_to_main_trace_segment": "Trace LDE + commitment",
        "commit_to_constraint_evaluations": "Composition LDE + commitment",
        "build_deep_composition_poly": "DEEP composition",
        "evaluate_deep_composition_poly": "DEEP evaluation",
        "compute_fri_layers": "FRI layers / remainder",
        "build_proof_object": "Queries + proof object",
    }
    items = [("Trace build (Rescue)", entry["trace_build_warm"]["mean_ns"] / 1e3)]
    items += [(names[key], value / 1e3) for key, value in entry["prove_stages_mean_ns"].items()
              if key in names]
    items.sort(key=lambda item: -item[1])
    width, left, top, row = 640, 200, 48, 28
    height = top + row * len(items) + 16
    plot_width = width - left - 80
    maximum = max(value for _, value in items)
    body = [title_text(16, f"Where time goes: {mode}, depth {depth} (µs, mean per proof)")]
    for index, (name, value) in enumerate(items):
        y = top + index * row
        bar = plot_width * value / maximum
        body.append(f"<text x='{left - 10}' y='{y + 14}' font-size='12' fill='{MUTED}' "
                    f"text-anchor='end'>{escape(name)}</text>"
                    f"<rect x='{left}' y='{y}' width='{max(bar, 2)}' height='18' rx='3' "
                    f"fill='{PALETTE[0]}'/><text x='{left + bar + 6}' y='{y + 14}' font-size='11' "
                    f"fill='{TEXT}'>{label_number(value)}</text>")
    return svg(width, height, "".join(body))


def main():
    directory = pathlib.Path(sys.argv[1])
    summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))["summary"]
    output = directory / "charts"
    output.mkdir(exist_ok=True)

    def save(name, chart):
        if chart:
            (output / f"{name}.svg").write_text(chart, encoding="utf-8")

    for name, *spec in BAR_CHARTS:
        save(name, bar_chart(summary, *spec))
    for name, *spec in LINE_CHARTS:
        save(name, scaling_chart(summary, *spec))
    for depth in (7, 15, 31):
        save(f"sweep_proof_size_depth{depth}",
             sweep_chart(summary, "Proof size by FRI shape", "proof_size_bytes", "KiB", 1024, depth))
        save(f"sweep_verify_depth{depth}",
             sweep_chart(summary, "Verification by FRI shape", "verify_median_ns", "µs", 1e3, depth))
    stages_file = directory / "stages.json"
    if stages_file.exists():
        stages = json.loads(stages_file.read_text(encoding="utf-8"))
        for mode in ("baseline", "optimized"):
            for depth in (7, 31):
                save(f"stages_{mode}_depth{depth}", stage_chart(stages, mode, depth))
    print(output)


if __name__ == "__main__":
    main()
