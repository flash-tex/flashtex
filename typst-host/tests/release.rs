//! Image and island ids no held page uses are released and rebound
//! (spec §5): a reflow that moves an island on every edit does not grow
//! the connection's image table (or the client's store) without bound.

mod common;

use common::*;
use flashtex_display_list::json::Json;
use flashtex_typst_host::convert::{self, ClientCaps, Positions, Tables};
use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};

#[test]
fn moving_islands_do_not_grow_the_image_table() {
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let caps = ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        image_data: true,
        ..Default::default()
    };
    let root = project("release", "");
    let mut tables = Tables::new();
    let mut max_id = 0;
    for k in 0..40 {
        // The gradient moves down by one line per edit: a new island key.
        let src = format!(
            "#set page(width: 120pt, height: 120pt)\n{}#rect(width: 30pt, height: 10pt, fill: gradient.linear(red, blue))\n",
            "x\\\n".repeat(k % 6)
        );
        std::fs::write(root.join("main.typ"), src).unwrap();
        let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
        let doc = typst::compile::<typst_layout::PagedDocument>(&world)
            .output
            .unwrap();
        let pp = flashtex_typst_host::pdfpos::derive(&doc, &[0]).unwrap();
        tables.begin_compile();
        let out = convert::page(
            &world,
            &doc,
            0,
            &mut tables,
            caps,
            &[],
            Positions::Pdf(&pp[0]),
        )
        .unwrap();
        for (info, _) in &out.images {
            if let Some(Json::Int(id)) = json_of(info).get("id") {
                max_id = max_id.max(*id);
            }
        }
        tables.release_images(doc.pages().len());
        assert!(
            tables.image_count() <= 1,
            "edit {k}: {} images bound",
            tables.image_count()
        );
    }
    // Two ids at most: the page the client holds still uses the old
    // island while the new one is bound; the old id is then released and
    // rebound by the next move. 40 moves, not 40 ids.
    assert_eq!(max_id, 2);
}
