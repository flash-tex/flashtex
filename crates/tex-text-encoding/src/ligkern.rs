//! TeX's main-loop ligature/kern construction (tex.web §1034–1040), transcribed
//! as a state machine over one maximal run of character tokens in one font.
//!
//! A run ends where TeX's `main_loop_lookahead` meets a token that is not a
//! character (`letter`, `other_char`, `char_given`, `char_num`): spaces, groups,
//! `\leavevmode`, font changes, `\kern`, ... The caller splits the input into
//! runs and passes whether the run starts after `\noboundary` and ends at one.

use crate::tfm::{ScaledFont, KERN_FLAG, STOP_FLAG};
use std::collections::VecDeque;

const NON_CHAR: i32 = 256;

/// An item produced for the horizontal list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunItem {
    /// A character node; `ligature` holds the original characters for a ligature node.
    Char { code: u8, ligature: Option<Vec<u8>> },
    /// A font kern from the lig/kern program.
    Kern(i32),
    /// The empty discretionary TeX inserts after the hyphen char in unrestricted
    /// horizontal mode (§1039 `ins_disc`).
    Disc,
}

#[derive(Debug, Clone, Copy)]
struct StackItem {
    ch: i32,
    /// true = a character node; false = a lig item (§1039 `new_lig_item`).
    is_char: bool,
    lig_ptr: Option<u8>,
}

/// Options for [`lig_kern_run`].
#[derive(Debug, Clone, Copy)]
pub struct RunOptions {
    /// `false` when the run follows `\noboundary` (TeX's `cancel_boundary`).
    pub left_boundary: bool,
    /// `false` when the run is terminated by `\noboundary`.
    pub right_boundary: bool,
    /// Insert empty discretionaries after `\hyphenchar` (mode > 0, i.e. paragraph
    /// horizontal mode; `false` inside `\hbox`).
    pub unrestricted_hmode: bool,
    /// `\hyphenchar` of the font (LaTeX sets 45 for text fonts).
    pub hyphen_char: i32,
}

impl Default for RunOptions {
    fn default() -> Self {
        RunOptions {
            left_boundary: true,
            right_boundary: true,
            unrestricted_hmode: false,
            hyphen_char: 45,
        }
    }
}

enum Label {
    Wrapup,
    Move,
    Move1,
    Move2,
    Lookahead,
    LigLoop,
    LigLoop1,
    MoveLig,
}

/// Runs TeX's ligature/kern algorithm over `chars` (character codes in `font`).
pub fn lig_kern_run(font: &ScaledFont, chars: &[u8], opts: RunOptions) -> Vec<RunItem> {
    let mut out: Vec<RunItem> = Vec::new();
    if chars.is_empty() {
        return out;
    }
    let mut pos = 1usize;
    let mut bchar: i32 = font.bchar.map_or(NON_CHAR, |b| b as i32);
    let false_bchar: i32 = match font.bchar {
        Some(b) if !font.char_exists(b) => b as i32,
        _ => NON_CHAR,
    };
    let mut stack: VecDeque<StackItem> = VecDeque::new();
    stack.push_front(StackItem {
        ch: chars[0] as i32,
        is_char: true,
        lig_ptr: None,
    });
    let mut cur_l: i32 = chars[0] as i32;
    let mut cur_r: i32 = NON_CHAR;
    let mut cur_q: usize = out.len();
    let mut ligature_present = false;
    let mut ins_disc = false;
    let mut main_k: usize = 0;

    let mut label = match font.bchar_label {
        Some(k) if opts.left_boundary => {
            cur_r = cur_l;
            cur_l = NON_CHAR;
            main_k = k;
            Label::LigLoop1
        }
        _ => Label::Move2,
    };

    // wrapup(#): §1035.
    let wrapup = |out: &mut Vec<RunItem>,
                  cur_l: i32,
                  cur_q: usize,
                  ligature_present: &mut bool,
                  ins_disc: &mut bool| {
        if cur_l < NON_CHAR {
            if out.len() > cur_q {
                if let Some(RunItem::Char { code, .. }) = out.last() {
                    if *code as i32 == opts.hyphen_char {
                        *ins_disc = true;
                    }
                }
            }
            if *ligature_present {
                let originals: Vec<u8> = out
                    .drain(cur_q..)
                    .flat_map(|it| match it {
                        RunItem::Char {
                            code,
                            ligature: None,
                        } => vec![code],
                        RunItem::Char {
                            ligature: Some(o), ..
                        } => o,
                        _ => vec![],
                    })
                    .collect();
                out.push(RunItem::Char {
                    code: cur_l as u8,
                    ligature: Some(originals),
                });
                *ligature_present = false;
            }
            if *ins_disc {
                *ins_disc = false;
                if opts.unrestricted_hmode {
                    out.push(RunItem::Disc);
                }
            }
        }
    };

    let mut guard = 0usize;
    loop {
        guard += 1;
        if guard > 1_000_000 {
            break; // infinite ligature loop protection (TeX: check_interrupt)
        }
        match label {
            Label::Wrapup => {
                wrapup(&mut out, cur_l, cur_q, &mut ligature_present, &mut ins_disc);
                label = Label::Move;
            }
            Label::Move => {
                if stack.is_empty() {
                    return out; // goto reswitch: the terminating token is processed by the caller
                }
                cur_q = out.len();
                cur_l = stack[0].ch;
                label = Label::Move1;
            }
            Label::Move1 => {
                if !stack[0].is_char {
                    label = Label::MoveLig;
                } else {
                    label = Label::Move2;
                }
            }
            Label::Move2 => {
                let c = stack[0].ch;
                if c >= NON_CHAR || !font.char_exists(c as u8) {
                    // char_warning; goto big_switch: restart with the remaining input.
                    stack.pop_front();
                    let rest = &chars[pos..];
                    out.extend(lig_kern_run(font, rest, opts));
                    return out;
                }
                stack.pop_front();
                out.push(RunItem::Char {
                    code: c as u8,
                    ligature: None,
                });
                label = Label::Lookahead;
            }
            Label::Lookahead => {
                if pos < chars.len() {
                    let c = chars[pos] as i32;
                    pos += 1;
                    stack.push_front(StackItem {
                        ch: c,
                        is_char: true,
                        lig_ptr: None,
                    });
                    cur_r = if c == false_bchar { NON_CHAR } else { c };
                } else {
                    if !opts.right_boundary {
                        bchar = NON_CHAR;
                    }
                    cur_r = bchar;
                    stack.clear();
                }
                label = Label::LigLoop;
            }
            Label::LigLoop => {
                if cur_l >= NON_CHAR {
                    label = Label::Wrapup;
                    continue;
                }
                match font.lig_kern_start(cur_l as u8) {
                    None => label = Label::Wrapup,
                    Some(_) if cur_r == NON_CHAR => label = Label::Wrapup,
                    Some(k) => {
                        main_k = k;
                        label = Label::LigLoop1;
                    }
                }
            }
            Label::LigLoop1 => {
                let Some(step) = font.step(main_k) else {
                    label = Label::Wrapup;
                    continue;
                };
                if step.next as i32 == cur_r && step.skip <= STOP_FLAG {
                    // §1040
                    if step.op >= KERN_FLAG {
                        wrapup(&mut out, cur_l, cur_q, &mut ligature_present, &mut ins_disc);
                        out.push(RunItem::Kern(font.kern_value(step)));
                        label = Label::Move;
                        continue;
                    }
                    match step.op {
                        1 | 5 => {
                            cur_l = step.rem as i32;
                            ligature_present = true;
                        }
                        2 | 6 => {
                            cur_r = step.rem as i32;
                            if stack.is_empty() {
                                stack.push_front(StackItem {
                                    ch: cur_r,
                                    is_char: false,
                                    lig_ptr: None,
                                });
                                bchar = NON_CHAR;
                            } else if stack[0].is_char {
                                let orig = stack[0].ch as u8;
                                stack[0] = StackItem {
                                    ch: cur_r,
                                    is_char: false,
                                    lig_ptr: Some(orig),
                                };
                            } else {
                                stack[0].ch = cur_r;
                            }
                        }
                        3 => {
                            cur_r = step.rem as i32;
                            stack.push_front(StackItem {
                                ch: cur_r,
                                is_char: false,
                                lig_ptr: None,
                            });
                        }
                        7 | 11 => {
                            wrapup(&mut out, cur_l, cur_q, &mut ligature_present, &mut ins_disc);
                            cur_q = out.len();
                            cur_l = step.rem as i32;
                            ligature_present = true;
                        }
                        _ => {
                            cur_l = step.rem as i32;
                            ligature_present = true;
                            if stack.is_empty() {
                                label = Label::Wrapup;
                            } else {
                                label = Label::Move1;
                            }
                            continue;
                        }
                    }
                    if step.op > 4 && step.op != 7 {
                        label = Label::Wrapup;
                        continue;
                    }
                    if cur_l < NON_CHAR {
                        label = Label::LigLoop;
                        continue;
                    }
                    main_k = font.bchar_label.unwrap_or(usize::MAX);
                    label = Label::LigLoop1;
                    continue;
                }
                if step.skip == 0 {
                    main_k += 1;
                } else {
                    if step.skip >= STOP_FLAG {
                        label = Label::Wrapup;
                        continue;
                    }
                    main_k += step.skip as usize + 1;
                }
            }
            Label::MoveLig => {
                let item = stack.pop_front().expect("lig item");
                let main_p = item.lig_ptr;
                if let Some(orig) = main_p {
                    out.push(RunItem::Char {
                        code: orig,
                        ligature: None,
                    });
                }
                ligature_present = true;
                if stack.is_empty() {
                    if main_p.is_some() {
                        label = Label::Lookahead;
                        continue;
                    }
                    cur_r = bchar;
                } else {
                    cur_r = stack[0].ch;
                }
                label = Label::LigLoop;
            }
        }
    }
    out
}
