import json, sys
order = ["key_to_edit_hook", "edit_hook_to_sent", "sent_to_frame_read", "host_first_page", "decode", "prepare", "raster", "raster_to_commit",
         "to_main", "main_to_install", "ca_commit_flush", "main_to_commit", "commit_to_vsync", "key_to_commit", "key_to_vsync"]
for f in sys.argv[1:]:
    j = json.load(open(f))
    st = j["stages"]
    print(f.split("/")[-1], "pages", j["pages"], "samples", j["samples"], "fast", j.get("fastEdits"))
    print("   " + "  ".join("%s %.2f/%.2f" % (k, st[k]["p50"], st[k]["p95"]) for k in order if k in st))
