import json, os, sys, statistics
base = '/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad/a38-bench/'
for label in sys.argv[1:]:
    p = base + label + '/scroll.json'
    if not os.path.exists(p):
        print(label, 'NO RESULT'); continue
    d = json.load(open(p))
    ft = d.get('first_tile_ms', [])
    fts = f"first-tile p50 {statistics.median(ft):.0f} max {max(ft):.0f} ms (n={len(ft)})" if ft else "first-tile -"
    print(f"{label}: pages {d['pages']} ppp {d['px_per_pt']} pane {d.get('pane')} frames {d['frames']} dropped {d['dropped_frames']} max {d['interval_ms_max']:.1f} "
          f"missing {d['frames_with_missing_visible_tiles']} bitmaps {d['bitmap_bytes_max']/1e6:.0f}MB footprint {d['footprint_bytes_max']/1e6:.0f}MB "
          f"clipped {d.get('cut_raster_resident_bytes_max', 0)/1e6:.0f}MB kept {d.get('kept_page_raster_bytes_max', 0)/1e6:.0f}MB "
          f"jobs {d['tile_jobs_off_main']} tiles {d['tiles_rastered_off_main']} skipped {d.get('tiles_skipped_undrawn')} jobmax {d['tile_job_ms_max']:.1f} "
          f"{fts} load {[round(x) for x in d['load_average_start']]}")
