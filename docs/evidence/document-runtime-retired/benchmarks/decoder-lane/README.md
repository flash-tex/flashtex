# Decoder worker actual-helper evidence

Unchanged521792-byte source,20edits targeting30ms, explicit15MiB compiler frames,
same pinned original compiler. First pair: defaultfinal644.05ms/lastACK190.37ms;
negotiatedfinal655.77ms/lastACK157.94ms. Default delivers one final current frame;
negotiated delivers one historical and one current. Every token/version check,
all40ACKs, independent full clean finals and killed-helper reopens pass.

Nonempty owner polls in these samples take roughly13–51ms for actual responses,
versus earlier115–190ms samples. Parsing now runs on the decoder worker; validation
and discarded-result destruction still consume owner time. This is attribution
across separate runs, not a stable causal end-to-end ratio. Final delivery still
exceeds200ms and native painting is not measured.

A third unchanged default run adds helper-only Linux memory evidence: peak RSS
224396kB, currentRSS135608kB,7OSthreads; final618.45ms, another20ACKs/cleanfinal/reopen
pass. This includes helper allocator/history/result state, not compiler or Python
consumer memory. There is no matched RSS baseline and no claimed process-RSS cap.
Raw queue capacity and simultaneous in-runtime decoding counts remain bounded;
parsed Value expansion and worker stack overhead are separate from raw byte limits.
