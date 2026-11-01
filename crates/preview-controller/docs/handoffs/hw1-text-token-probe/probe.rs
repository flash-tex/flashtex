#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DocumentId(pub usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span { pub document: DocumentId, pub start: usize, pub end: usize }
impl Span { pub fn in_document(document: DocumentId, start: usize, end: usize)->Self { Self {document,start,end} } }
#[path="/home/natkarri/flashtex-captures/hw1-logical-1c02a2bc/crates/compiler/src/lexer.rs"]
mod lexer;
fn main() {
 let cases = [r"\text{and}", "\\text{a  b}", "\\text{a%comment\nb}", r"\text{a{b}c}", r"\text{\{x\}}", r"\text{\alpha}", r"\text{é}", "\\text{a\n\nb}", r"\text x", r"\text{a"];
 for source in cases {
  println!("SOURCE {source:?}");
  for t in lexer::tokenize(source) {
   assert!(source.get(t.span.start..t.span.end).is_some());
   println!("{:?} {}..{} {:?}", t.kind,t.span.start,t.span.end,&source[t.span.start..t.span.end]);
  }
 }
}
