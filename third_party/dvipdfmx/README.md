# `third_party/dvipdfmx`

Provenance record for the parts of TeX Live 2026's **dvipdfm-x**
(`xdvipdfmx`, the PDF driver XeTeX pipes its XDV output to) that
`crates/flashtex-xetex/src/out/` ports to Rust. Unicode mode's output
(`docs/design/xetex/PLAN.md` §3.2) writes its PDF from the display list with
FlashTeX's own writer; where it must match what xdvipdfmx writes, individual
routines follow dvipdfm-x's C, and each such routine names the C it ports.
No dvipdfm-x source is vendored, compiled or linked: this directory holds
only this record, with the sha256 of each upstream file the port follows.

## Pin

| | |
|---|---|
| Upstream | TeX Live source tree, GitHub mirror <https://github.com/TeX-Live/texlive-source>, `texk/dvipdfm-x/` |
| Tag | `tags/texlive-2026.1` ("texlive-2026.1 tag based on r78399") |
| Commit | `6a300188053b8f2ded89dbd52293732a706b9c0e` (the same pin as `third_party/xetex`, `third_party/icu`, `third_party/harfbuzz`, `third_party/teckit`) |
| Version | 20260317 (`AC_INIT([dvipdfm-x (TeX Live)], [20260317], ...)` in `texk/dvipdfm-x/configure.ac`) |
| Fetched | 2026-11-15 |

## Licence

GNU General Public License, version 2 or (at your option) any later version
(GPL-2.0-or-later): every file below carries that grant, and
`texk/dvipdfm-x/COPYING` (sha256
`dcc100d4161cc0b7177545ab6e47216f84857cda3843847c792a25289852dcaa`) is the
GPL v2 text. The copyright holders the files name:

- Copyright (C) 2002-2026 by Jin-Hwan Cho, Matthias Franz and Shunsaku
  Hirata, the dvipdfmx project team;
- Copyright (C) 1998, 1999 by Mark A. Wicks (dvipdfm);
- Copyright (C) 2012-2015 by Khaled Hosny (`dvi.c`);
- Copyright (c) 2006 SIL (xdvipdfmx extensions for XeTeX support,
  `dvipdfmx.c`).

`crates/flashtex-xetex` is itself GPL-2.0-or-later (its `Cargo.toml` and
`LICENSE`), so the port is compatible with it; that crate's `LICENSE`
records the port and these notices. No MIT crate may contain or link this
code (DESIGN.md §3; `scripts/check-license-boundary.sh` keeps MIT crates off
the GPL crates' dependency graph).

## Ported files

Upstream file (`texk/dvipdfm-x/`) → the FlashTeX file whose routines follow
it, and what they follow.

| upstream | sha256 | ported in `crates/flashtex-xetex/src/out/` |
|---|---|---|
| `agl.c` | `d773dc24d4fa2a7110142292f3905b2a93c2c913bb4929c3c0ee392c7d4435a1` | `agl.rs`: `agl_init_map`, `agl_get_unicodes`, `agl_sput_UTF16BE` |
| `tt_cmap.c` | `54df60f65d869b00b6b2653d88aeef30804d016058c025127691c0320cb7de12` | `tounicode.rs`: `otf_create_ToUnicode_stream`, `create_inverse_cmap4`/`12`, `is_PUA_or_presentation`; `agl.rs` (its glyph-name fallback) |
| `tt_gsub.c` | `9291100f3272ec4f600505cf30f30934882b7599047a2e608266914cfd869b0a` | `tounicode.rs`: `otl_gsub_add_ToUnicode` |
| `tt_post.c` | `ce4a75f56adf5428d588b20938bfc35e502300f38a718ca8f803a3c4cc81de11` | `tounicode.rs`: `macglyphorder[]` |
| `type0.c` | `cda48a5427fea740a19b549662fda25cdfac4ec2d84a273017b3e73f22f2f2ce` | `tounicode.rs`: `Type0Font_attach_ToUnicode_stream` |
| `pdfencoding.c` | `b54be0b3fbb03e9356fd0cd1ece011320897f254b0ffbb15e132bc4cbde66a26` | `tounicode.rs`: `pdf_create_ToUnicode_CMap`; `agl.rs` |
| `cmap_write.c` | `75b2ab5050cbdf48a403bc82052029f859faed50d3ce8ed73544cdb2c62294ad` | `tounicode.rs`: `CMap_create_stream`, `write_name` |
| `cff_dict.c` | `6a5cca52283fed9985350582457fc796497fe8f05d44dc3d19a96d26a53bd260` | `cffcid.rs`: `get_real` |
| `cidtype0.c` | `947b5fd7bd104d26144cd7c1739357537faeac0c0aa18e10bae742d1fcf791f2` | `pdf.rs`: a CFF font's name |
| `type1.c` | `8267357a90bf8c74a7b4068426f27466140a128417cf60a4e874f69e3492a5a7` | `pdf.rs`: `add_metrics`, `get_font_attr` |
| `tt_table.c` | `1c6a38f9b69d965771c2395fb4930a3399add92aeac714dda2796ceb4ca54301` | `pdf.rs`: `tt_read_head_table` |
| `tt_aux.c` | `7a6943f72d2170b80327ed7100bce07ce84b03f91e9a7f4d63bebacb8fefaa40` | `pdf.rs`: `tt_get_fontdesc` |
| `pngimage.c` | `cdb969b406dcb16dd79f9ab90dcaa4c750b691f221df9d940605580cdfdc717b` | `pdf.rs`: `check_transparency`, `create_soft_mask` |
| `pdfnames.c` | `8a6b56226ed0500ff689690cbd79a4b60191dcda04ffd1855aa15e6af6054dd6` | `pdf.rs`: `flat_table` |
| `pdfdoc.c` | `5df031b7df9e8599dc5ef0b059f3ffd93489dcddc65faf52eaf38db19522eaf4` | `pdf.rs`: the outline (`pdf_olitem`); `doc.rs`: `pdf_doc_add_goto` |
| `dvi.c` | `e0dffe935119b9d878ed6c11fe21048b996bf77e68c81d61c9e0cbab1937f909` | `doc.rs`: `catch_phantom`; `page.rs`: `compensation`, `dvi_set`/`dvi_put` for a TFM font, `dvi_vf_init`, `dvi_vf_finish`, `dvi_mark_depth`; `fonts.rs`: `dvi_locate_font` |
| `dvipdfmx.c` | `4f2a23af88c687f264103e14f0270fb208bfab0020e6b47b253d708a8d592e7e` | `doc.rs`, `page.rs`: the `C` option's 0x0010 flag |
| `spc_pdfm.c` | `254914baa5262d4a5008b801dd0cf1e1aa4cbcd3697154da65b33d4441778fd4` | `page.rs`: `set_rect_for_annot`, `pdf:bann`, `pdf:bcolor`/`scolor`; `special.rs`: the colours, `default_taintkeys`, `modify_strings`; `pdf.rs`: `spc_handler_pdfm_outline` |
| `spc_dvipdfmx.c` | `e3f069f267055ba46b947024b8899b843fdb28cbde0e644e2024fb831dc0498d` | `special.rs`: the `dvipdfmx:` specials |
| `specials.c` | `03ddb8848b10ea71451e52da36a519900bdbc141e76909cbafcdfcff11a8d050` | `special.rs`: the DVI position |
| `pdfximage.c` | `b47f205cc7911c941415901bf7cacc206bc0f5893cc09ef83751fb3f1f5ba7d1` | `special.rs`: `scale_to_fit_I`, `scale_to_fit_F` |
| `unicode.c` | `48718547ac5a9a8fe466e297a078160b5766f106e174376404eb3954e7494a2e` | `special.rs`: `UC_UTF8_decode_char` |
| `vf.c` | `de28f993f13bd95a5f82d1611685792affc720789b864cdfb8894a6abfe23f44` | `vf.rs`: `read_header`, `process_vf_file`, `read_a_char_def`, `get_pkt_signed_num`, `get_pkt_unsigned_num`, `vf_xxx`; `fonts.rs`: `vf_locate_font`, `read_a_font_def`; `page.rs`: `vf_set_char`, `vf_setrule`, `vf_putrule`, `vf_fnt` |
| `tfm.c` | `e6a0451c67a9bd00c8ac166d666b598c1475a0db40282668acc6ec0f51764df0` | `vf.rs`: `read_tfm`, `tfm_get_sizes`, `tfm_check_size`, `tfm_unpack_arrays`, `tfm_get_fw_width`, `tfm_get_fw_height`, `tfm_get_fw_depth` (TFM files only; not OFM or JFM) |
| `numbers.c` | `5a10703dc519ff918fb3ae6b0f3ee6c14f53fdb552eaa90f3c74fe2fbd191f80` | `vf.rs`: `sqxfw`, `get_unsigned_num`, `get_positive_quad` |

The list is the C files `src/out/` names (`grep -E '[a-z_0-9]+\.c\b'
crates/flashtex-xetex/src/out/*.rs`); a new port of a dvipdfm-x routine adds
its file here. To check a pin, fetch the file at the commit above and compare:

```sh
curl -fsSL https://raw.githubusercontent.com/TeX-Live/texlive-source/6a300188053b8f2ded89dbd52293732a706b9c0e/texk/dvipdfm-x/agl.c | shasum -a 256
```

ICU-derived parts of the crate are recorded in `third_party/icu`; this
record covers dvipdfm-x only.
