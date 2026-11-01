//! Read-only probe for exact peer f2fdb08; synthetic directory evidence, no font support claim.
use flashtex_font_engine::collection_layout;
fn be32(b:&mut[u8],at:usize,v:u32){b[at..at+4].copy_from_slice(&v.to_be_bytes());}
fn font(version:u32, table_start:u32)->Vec<u8>{
 let mut b=vec![0;128];b[..4].copy_from_slice(b"ttcf");be32(&mut b,4,version);be32(&mut b,8,1);be32(&mut b,12,32);
 be32(&mut b,32,0x10000);b[36..38].copy_from_slice(&1u16.to_be_bytes());b[44..48].copy_from_slice(b"cmap");be32(&mut b,52,table_start);be32(&mut b,56,8);b
}
fn main(){
 for (name,bytes) in [("valid_directory",font(0x10000,96)),("unsupported_ttc_version",font(0x90000,96)),("table_overlaps_header",font(0x10000,0)),("table_overlaps_directory",font(0x10000,32))]{
 println!("{name}: {}",if collection_layout(&bytes).is_ok(){"accepted"}else{"rejected"});
 }
 let mut b=font(0x10000,96);be32(&mut b,32,0xdeadbeef);
 println!("invalid_sfnt_version: {}",if collection_layout(&b).is_ok(){"accepted"}else{"rejected"});
}
