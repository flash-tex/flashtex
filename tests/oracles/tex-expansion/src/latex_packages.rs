//! The LaTeX package/class kernel (`ltclass.dtx`): reading `.sty`/`.cls`
//! files and the option-processing commands they use.
//!
//! LaTeX defines nearly all of this in TeX, so the definitions below are
//! copied from `latex.ltx` (TeX Live 2026, `ltclass.dtx` lines 18345-18960
//! and `lterror.dtx` 8795-8960) with two reductions: the file-hook and
//! `\@currpath` machinery (expl3-based, and only meaningful with kpathsea)
//! is dropped, and the two places where the real code needs the file
//! system or the terminal go through primitives of this crate:
//!
//! - `\usepackage`, `\RequirePackage`, `\documentclass` and `\LoadClass`
//!   are primitives ([`Engine::do_load_files`]) that read `[options]
//!   {names}[version]` exactly as `\@fileswith@pti@ns` does and then ask
//!   the host's *package reader* ([`Engine::set_package_reader`]) for each
//!   `name.ext`. A file the host returns is loaded through
//!   `\@onefilewithoptions` (the real code path: `\@pushfilename`,
//!   `\makeatletter`, option bookkeeping, the file itself, the
//!   `\AtEndOfPackage` hook, unprocessed-option errors, `\@popfilename`
//!   restoring the catcode of `@`). A name the host declines -- a package
//!   the typesetter models itself, or one that is simply not in the
//!   project -- is *passed through*: its `\ver@name.ext`/`\opt@name.ext`
//!   records are still made (so `\@ifpackageloaded` and option clashes
//!   behave), and the original `\usepackage[...]{...}` tokens are handed
//!   to the output untouched, exactly as before this module existed, so
//!   the typesetting layer sees them with their exact source spans.
//! - `\@latex@error`/`\@latex@warning` and the `\Package…`/`\Class…`
//!   message commands report through `\flashtex@latex@error` /
//!   `\flashtex@latex@warning`, which record a diagnostic at the current
//!   input position (the real ones write to the terminal and the log).
//!
//! What is deliberately not here: `\@currpath`, `\IfFileExists`,
//! `\InputIfFileExists` (the host resolves files), `\@filelist`,
//! `\@currnamestack`'s `\@expl…` hooks, `\@twoclasseserror` (the host
//! parser already keeps the first `\documentclass`), and the
//! `\currentgrouplevel` check in `\@fileswithoptions` (not modelled).

use std::rc::Rc;

use crate::catcode::CatCode;
use crate::expand::{Engine, Input, Pending, Step};
use crate::lexer::Lexer;
use crate::package_defs::{DeclaredOption, PackageDefinition, Provides};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Host callback resolving `name.ext` (`ext` is `sty` or `cls`, without the
/// dot) to the file's text. `None` declines: the command is passed through
/// to the typesetting layer. The host decides where files come from and
/// which packages its own models supersede.
pub type PackageReader = Rc<dyn Fn(&str, &str) -> Option<String>>;

/// A `.sty`/`.cls` file the engine opened, for the host's span mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedFile {
    /// The `source_id` of every token read from the file.
    pub source_id: u32,
    /// `name.ext` as the reader was asked for it.
    pub name: String,
    /// The `\usepackage`/`\documentclass`/... invocation that loaded it
    /// (synthetic when it was loaded from a macro body). Its `source_id`
    /// is the loader's: `0` for the document, another opened file's id for
    /// a nested `\RequirePackage`/`\LoadClass` -- the load chain.
    pub loaded_at: Span,
    /// `\ProvidesPackage`/`\ProvidesClass`, once the file has declared itself.
    pub provides: Option<Provides>,
    /// `\DeclareOption`s at the file's outermost level, in order.
    pub options: Vec<DeclaredOption>,
    /// Definitions made at the file's outermost level, in order (see
    /// `package_defs.rs`).
    pub definitions: Vec<PackageDefinition>,
}

/// Which loading command a [`Primitive::LoadFiles`](crate::scopes::Primitive)
/// stands for. The four differ only in the extension they look for and in
/// the name their pass-through form carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    UsePackage,
    RequirePackage,
    DocumentClass,
    LoadClass,
}

impl LoadKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            LoadKind::UsePackage => "usepackage",
            LoadKind::RequirePackage => "RequirePackage",
            LoadKind::DocumentClass => "documentclass",
            LoadKind::LoadClass => "LoadClass",
        }
    }

    fn is_class(self) -> bool {
        matches!(self, LoadKind::DocumentClass | LoadKind::LoadClass)
    }

    /// The extension macro `\@onefilewithoptions` compares against
    /// (`\ifx#4\@clsextension`) and expands inside `\csname`.
    fn extension_macro(self) -> &'static str {
        if self.is_class() {
            "@clsextension"
        } else {
            "@pkgextension"
        }
    }

    fn extension(self) -> &'static str {
        if self.is_class() {
            "cls"
        } else {
            "sty"
        }
    }

    /// The command the typesetting layer receives for a declined name.
    /// `\RequirePackage`/`\LoadClass` mean the same thing to it as
    /// `\usepackage`/`\documentclass`, and it only models the latter.
    fn pass_through_name(self) -> &'static str {
        if self.is_class() {
            "documentclass"
        } else {
            "usepackage"
        }
    }
}

/// `\NeedsTeXFormat`, `\ProvidesPackage`, `\ProvidesClass` and
/// `\ProvidesFile`: kernel macros inside a package or class file, and
/// pass-through everywhere else. The host parser models them in a document
/// (as inert metadata with its own argument diagnostics), and the real
/// `\NeedsTeXFormat` answers a bad argument with `\endinput`, which would
/// end the document being typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declaration {
    NeedsTeXFormat,
    ProvidesPackage,
    ProvidesClass,
    ProvidesFile,
}

impl Declaration {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Declaration::NeedsTeXFormat => "NeedsTeXFormat",
            Declaration::ProvidesPackage => "ProvidesPackage",
            Declaration::ProvidesClass => "ProvidesClass",
            Declaration::ProvidesFile => "ProvidesFile",
        }
    }
}

/// The package/class kernel, run once after `prelude.rs`'s `PRELUDE` when
/// the initial state is built. Line references are to `latex.ltx` of TeX
/// Live 2026 where a definition is copied.
pub(crate) const PACKAGES_PRELUDE: &str = r"\makeatletter
\def\fmtname{LaTeX2e}
\def\fmtversion{2025-11-01}
\def\MessageBreak{ }
\def\@spaces{\space\space\space\space}
\def\@expandtwoargs#1#2#3{\protected@edef\reserved@a{\noexpand#1{#2}{#3}}\reserved@a}
\def\@removeelement#1#2#3{%
  \def\reserved@a##1,#1,##2\reserved@a{##1,##2\reserved@b}%
  \def\reserved@b##1,\reserved@b##2\reserved@b{%
    \ifx,##1\@empty\else##1\fi}%
  \edef#3{%
    \expandafter\reserved@b\reserved@a,#2,\reserved@b,#1,\reserved@a}}
\def\in@#1#2%
 {%
   \begingroup
     \def\in@@##1#1{}%
     \toks@\expandafter{\in@@#2{}{}#1}%
     \edef\in@@{\the\toks@}%
   \expandafter\endgroup
   \ifx\in@@\@empty
     \in@false
   \else
     \in@true
   \fi
 }
\newif\ifin@
\def\zap@space#1 #2{%
  #1%
  \ifx#2\@empty\else\expandafter\zap@space\fi
  #2}
\def\@latex@error#1#2{\flashtex@latex@error{LaTeX Error: #1}}
\def\@latex@warning#1{\flashtex@latex@warning{LaTeX Warning: #1}}
\let\@latex@warning@no@line\@latex@warning
\def\@latex@info#1{}
\let\@latex@info@no@line\@latex@info
\let\@warning\@latex@warning
\let\@@warning\@latex@warning@no@line
\let\@latexerr\@latex@error
\def\PackageError#1#2#3{\flashtex@latex@error{Package #1 Error: #2}}
\def\PackageWarning#1#2{\flashtex@latex@warning{Package #1 Warning: #2}}
\let\PackageWarningNoLine\PackageWarning
\def\PackageInfo#1#2{}
\let\PackageNote\PackageInfo
\let\PackageNoteNoLine\PackageInfo
\def\ClassError#1#2#3{\flashtex@latex@error{Class #1 Error: #2}}
\def\ClassWarning#1#2{\flashtex@latex@warning{Class #1 Warning: #2}}
\let\ClassWarningNoLine\ClassWarning
\def\ClassInfo#1#2{}
\let\ClassNote\ClassInfo
\let\ClassNoteNoLine\ClassInfo
\def\typeout#1{}
\def\wlog#1{}
\let\@declaredoptions\@empty
\let\@classoptionslist\relax
\let\@raw@classoptionslist\relax
\let\@unusedoptionlist\@empty
\let\CurrentOption\@empty
\let\@currname\@empty
\global\let\@currext=\@empty
\def\@clsextension{cls}
\def\@pkgextension{sty}
\def\@pushfilename{%
  \xdef\@currnamestack{%
    {\@currname}%
    {\@currext}%
    {\the\catcode`\@}%
    \@currnamestack}}
\def\@popfilename{\expandafter\@p@pfilename\@currnamestack\@nil}
\def\@p@pfilename#1#2#3#4\@nil{%
  \gdef\@currname{#1}%
  \gdef\@currext{#2}%
  \catcode`\@#3\relax
  \gdef\@currnamestack{#4}}
\gdef\@currnamestack{}
\def\@ptionlist#1{%
  \@ifundefined{opt@#1}\@empty{\csname opt@#1\endcsname}}
\def\@ifpackageloaded{\@ifl@aded\@pkgextension}
\def\@ifclassloaded{\@ifl@aded\@clsextension}
\def\@ifl@aded#1#2{%
  \expandafter\ifx\csname ver@#2.#1\endcsname\relax
    \expandafter\@secondoftwo
  \else
    \expandafter\@firstoftwo
  \fi}
\def\@ifpackagelater{\@ifl@ter\@pkgextension}
\def\@ifclasslater{\@ifl@ter\@clsextension}
\def\IfFormatAtLeastTF{\@ifl@t@r\fmtversion}
\let\IfPackageAtLeastTF\@ifpackagelater
\let\IfClassAtLeastTF\@ifclasslater
\def\IfFileAtLeastTF#1{\expandafter\@ifl@t@r\csname ver@#1\endcsname}
\def\@ifl@ter#1#2{%
  \expandafter\@ifl@t@r
    \csname ver@#2.#1\endcsname}
\def\@ifl@t@r#1#2{%
  \ifnum\expandafter\@parse@version@#1//00\@nil<%
        \expandafter\@parse@version@#2//00\@nil
    \expandafter\@secondoftwo
  \else
    \expandafter\@firstoftwo
  \fi}
\def\@parse@version@#1{\@parse@version0#1}
\def\@parse@version#1/#2/#3#4#5\@nil{%
\@parse@version@dash#1-#2-#3#4\@nil
}
\def\@parse@version@dash#1-#2-#3#4#5\@nil{%
  \if\relax#2\relax\else#1\fi#2#3#4 }
\def\@ifpackagewith{\@if@ptions\@pkgextension}
\def\@ifclasswith{\@if@ptions\@clsextension}
\def\@if@ptions#1#2{%
  \@expandtwoargs\@if@pti@ns{\@ptionlist{#2.#1}}}
\def\@if@pti@ns#1#2{%
 \let\reserved@a\@firstoftwo
 \edef\reserved@b{\zap@space#2 \@empty}%
 \@for\reserved@b:=\reserved@b\do{%
   \ifx\reserved@b\@empty
   \else
     \expandafter\in@\expandafter{\expandafter,\reserved@b,}{,#1,}%
     \ifin@
     \else
       \let\reserved@a\@secondoftwo
     \fi
   \fi
 }%
 \reserved@a}
\let \IfPackageLoadedTF            \@ifpackageloaded
\let \IfClassLoadedTF              \@ifclassloaded
\let \IfPackageLoadedWithOptionsTF \@ifpackagewith
\let \IfClassLoadedWithOptionsTF   \@ifclasswith
\def\IfPackageLoadedT   #1{\IfPackageLoadedTF{#1}\@firstofone\@gobble}
\def\IfPackageLoadedF   #1{\IfPackageLoadedTF{#1}{}}
\def\IfClassLoadedT     #1{\IfClassLoadedTF{#1}\@firstofone\@gobble}
\def\IfClassLoadedF     #1{\IfClassLoadedTF{#1}{}}
\def\IfPackageAtLeastT#1#2{\IfPackageAtLeastTF{#1}{#2}\@firstofone\@gobble}
\def\IfPackageAtLeastF#1#2{\IfPackageAtLeastTF{#1}{#2}{}}
\def\IfClassAtLeastT    #1#2{\IfClassAtLeastTF{#1}{#2}\@firstofone\@gobble}
\def\IfClassAtLeastF    #1#2{\IfClassAtLeastTF{#1}{#2}{}}
\def\IfFileAtLeastT   #1#2{\IfFileAtLeastTF{#1}{#2}\@firstofone\@gobble}
\def\IfFileAtLeastF   #1#2{\IfFileAtLeastTF{#1}{#2}{}}
\def\IfFormatAtLeastT   #1{\IfFormatAtLeastTF{#1}\@firstofone\@gobble}
\def\IfFormatAtLeastF   #1{\IfFormatAtLeastTF{#1}{}}
\def\IfPackageLoadedWithOptionsT #1#2{\IfPackageLoadedWithOptionsTF{#1}{#2}\@firstofone\@gobble}
\def\IfPackageLoadedWithOptionsF #1#2{\IfPackageLoadedWithOptionsTF{#1}{#2}{}}
\def\IfClassLoadedWithOptionsT #1#2{\IfClassLoadedWithOptionsTF{#1}{#2}\@firstofone\@gobble}
\def\IfClassLoadedWithOptionsF #1#2{\IfClassLoadedWithOptionsTF{#1}{#2}{}}
\def\IfFileLoadedTF#1{%
  \expandafter\ifx\csname ver@#1\endcsname\relax
    \expandafter\@secondoftwo
  \else
    \expandafter\@firstoftwo
  \fi}
\def\IfFileLoadedT  #1{\IfFileLoadedTF{#1}\@firstofone\@gobble}
\def\IfFileLoadedF  #1{\IfFileLoadedTF{#1}{}}
\def\flashtex@ProvidesPackage#1{%
  \xdef\@gtempa{#1}%
  \edef\reserved@a{\detokenize\expandafter{\@gtempa}}%
  \edef\reserved@b{\detokenize\expandafter{\@currname}}%
  \ifx\reserved@a\reserved@b\else
    \@latex@warning@no@line{You have requested
      \@cls@pkg\space`\@currname',
       but the \@cls@pkg\space provides `#1'}%
  \fi
  \@ifnextchar[\@pr@videpackage{\@pr@videpackage[]}}%]
\def\@pr@videpackage[#1]{%
  \expandafter\protected@xdef
     \csname ver@\@currname.\@currext\endcsname{#1}}
\let\flashtex@ProvidesClass\flashtex@ProvidesPackage
\def\flashtex@ProvidesFile#1{%
  \@ifnextchar[{\@providesfile{#1}}{\@providesfile{#1}[]}}
\def\@providesfile#1[#2]{%
    \expandafter\xdef\csname ver@#1\endcsname{#2}}
\def\@pass@ptions#1#2#3{%
  \expandafter\protected@xdef\csname opt@#3.#1\endcsname{%
    \@ifundefined{opt@#3.#1}\@empty
      {\csname opt@#3.#1\endcsname,}%
    \zap@space#2 \@empty}%
  \@ifundefined{@raw@opt@#3.#1}%
    {\expandafter\gdef\csname @raw@opt@#3.#1\expandafter\endcsname
              \expandafter{#2}}%
    {\expandafter\g@addto@macro\csname @raw@opt@#3.#1\expandafter\endcsname
              \expandafter{\expandafter,#2}}%
}
\def\PassOptionsToPackage{\@pass@ptions\@pkgextension}
\def\PassOptionsToClass{\@pass@ptions\@clsextension}
\def\DeclareOption{%
  \@ifstar\@defdefault@ds\@declareoption}
\long\def\@declareoption#1#2{%
   \xdef\@declaredoptions{\@declaredoptions,#1}%
   \toks@{#2}%
   \expandafter\edef\csname ds@#1\endcsname{\the\toks@}}
\long\def\@defdefault@ds#1{%
  \toks@{#1}%
  \edef\default@ds{\the\toks@}}
\def\@remove@eq@value#1=#2\@nil{#1}
\def\OptionNotUsed{%
  \ifx\@currext\@clsextension
    \xdef\@unusedoptionlist{%
      \ifx\@unusedoptionlist\@empty\else\@unusedoptionlist,\fi
      \expandafter\@remove@eq@value\CurrentOption=\@nil}%
  \fi}
\def\ProcessOptions{%
  \let\ds@\@empty
  \protected@edef\@curroptions{\@ptionlist{\@currname.\@currext}}%
  \@ifstar\@xprocess@ptions\@process@ptions}
\def\@process@ptions{%
  \@for\CurrentOption:=\@declaredoptions\do{%
    \ifx\CurrentOption\@empty\else
      \@expandtwoargs\in@{,\CurrentOption,}{%
         ,\ifx\@currext\@clsextension\else\@classoptionslist,\fi
         \@curroptions,}%
      \ifin@
        \@use@ption
        \expandafter\let\csname ds@\CurrentOption\endcsname\@empty
      \fi
    \fi}%
  \@process@pti@ns}
\def\@xprocess@ptions{%
  \ifx\@currext\@clsextension\else
   \ifx\@classoptionslist\relax\else
    \@for\CurrentOption:=\@classoptionslist\do{%
      \ifx\CurrentOption\@empty\else
        \@ifundefined{ds@\detokenize\expandafter{\CurrentOption}}{}{%
          \@use@ption
          \expandafter\let\csname ds@\CurrentOption\endcsname\@empty
        }%
      \fi}%
    \fi
  \fi
  \@process@pti@ns}
\def\@process@pti@ns{%
  \@for\CurrentOption:=\@curroptions\do{%
    \@ifundefined{ds@\detokenize\expandafter{\CurrentOption}}%
      {\@use@ption
       \default@ds}%
      \@use@ption}%
  \@for\CurrentOption:=\@declaredoptions\do{%
    \expandafter\let\csname ds@\CurrentOption\endcsname\relax}%
  \let\CurrentOption\@empty
  \AtEndOfPackage{\expandafter\let
                     \csname unprocessedoptions-\@currname.\@currext\endcsname
                     \relax}}
\def\@options{\ProcessOptions*}
\def\@use@ption{%
  \@expandtwoargs\@removeelement
     {\expandafter\@remove@eq@value\CurrentOption=\@nil}%
  \@unusedoptionlist\@unusedoptionlist
  \csname ds@\detokenize\expandafter{\CurrentOption}\endcsname}
\def\ExecuteOptions#1{%
  \edef\@fortmp{\zap@space#1 \@empty}%
  \def\reserved@a##1\@nil{%
    \@for\CurrentOption:=\@fortmp\do
             {\csname ds@\CurrentOption\endcsname}%
    \edef\CurrentOption{##1}}%
  \expandafter\reserved@a\CurrentOption\@nil}
\def\@loadwithoptions#1#2#3{%
  \expandafter\let\csname opt@#3.#1\expandafter\endcsname
       \csname opt@\@currname.\@currext\endcsname
  \expandafter\let\csname @raw@opt@#3.#1\expandafter\endcsname
       \csname @raw@opt@\@currname.\@currext\endcsname
   #2{#3}}
\def\LoadClassWithOptions{%
  \@loadwithoptions\@clsextension\LoadClass}
\def\RequirePackageWithOptions{%
  \AtEndOfPackage{\expandafter\let
                    \csname unprocessedoptions-\@currname.\@currext\endcsname
                    \relax}%
  \@loadwithoptions\@pkgextension\RequirePackage}
\def\flashtex@NeedsTeXFormat#1{%
  \def\reserved@a{#1}%
  \ifx\reserved@a\fmtname
    \expandafter\@needsformat
  \else
     \@latex@error{This file needs format `\reserved@a'%
       \MessageBreak but this is `\fmtname'}{%
       The current input file will not be processed
       further,\MessageBreak
       because it was written for some other flavor of
       TeX.\MessageBreak\@ehd}%
     \endinput \fi}
\def\@needsformat{%
  \@ifnextchar[%]
    \@needsf@rmat
    {}}
\def\@needsf@rmat[#1]{%
    \@ifl@t@r\fmtversion{#1}{}%
    {\@latex@warning@no@line
        {You have requested release `#1' of LaTeX,\MessageBreak
         but only release `\fmtversion' is available}}}
\def\flashtex@setclassoptions#1{%
  \ifx\@classoptionslist\relax
    \protected@xdef\@classoptionslist{\zap@space#1 \@empty}%
    \gdef\@raw@classoptionslist{#1}%
  \fi}
\def\@onefilewithoptions#1[#2][#3]#4{%
  \@pushfilename
  \xdef\@currname{#1}%
  \global\let\@currext#4%
  \@ifl@aded\@currext\@currname
    {\@onefilewithoptions@clashchk{#2}}%
    {\makeatletter
     \@reset@ptions
     \load@onefile@withoptions{#2}%
     \@firstofone}%
    {\@ifl@ter\@currext{\@currname}{#3}{}%
      {\@latex@warning@no@line
        {You have requested,\on@line,
         version\MessageBreak
           `#3' of \@cls@pkg\space \@currname,\MessageBreak
         but only version\MessageBreak
          `\csname ver@\@currname.\@currext\endcsname'\MessageBreak
         is available}}%
     \ifx\@currext\@clsextension\let\LoadClass\@twoloadclasserror\fi}%
    \@popfilename
    \@reset@ptions}
\def\on@line{}
\def\@onefilewithoptions@clashchk#1{%
  \@if@ptions\@currext{\@currname}{#1}{}%
      {\@latex@error
        {Option clash for \@cls@pkg\space \@currname}%
        {The package \@currname\space has already been loaded
         with options:\MessageBreak
         \space\space[\@ptionlist{\@currname.\@currext}]\MessageBreak
         There has now been an attempt to load it
          with options\MessageBreak
         \space\space[#1]\MessageBreak
         Adding the global options:\MessageBreak
         \space\space
              \@ptionlist{\@currname.\@currext},#1\MessageBreak
         to your \noexpand\documentclass declaration may fix this.%
         \MessageBreak
         Try typing \space <return> \space to proceed.}}%
     \@firstofone}
\let\@unprocessedoptions\@undefined
\def\load@onefile@withoptions#1{%
  \let\CurrentOption\@empty
  \@reset@ptions
  \@pass@ptions\@currext{#1}{\@currname}%
  \global\expandafter
  \let\csname ver@\@currname.\@currext\endcsname\@empty
  \expandafter\let\csname\@currname.\@currext-h@@k\endcsname\@empty
  \flashtex@inputfile{\@currname}{\@currext}%
  \expandafter\let\csname unprocessedoptions-\@currname.\@currext\endcsname
                  \@@unprocessedoptions
  \csname\@currname.\@currext-h@@k\endcsname
  \expandafter\let\csname\@currname.\@currext-h@@k\endcsname
            \@undefined
  \ifx\@unprocessedoptions\relax
    \let\@unprocessedoptions\@undefined
  \else
    \csname unprocessedoptions-\@currname.\@currext\endcsname
  \fi
  \expandafter\let
      \csname unprocessedoptions-\@currname.\@currext\endcsname
     \@undefined}
\def\@reset@ptions{%
  \global\ifx\@currext\@clsextension
    \let\default@ds\OptionNotUsed
   \else
    \let\default@ds\@unknownoptionerror
  \fi
  \global\let\ds@\@empty
  \global\let\@declaredoptions\@empty}
\def\AtEndOfPackage{%
  \expandafter\g@addto@macro\csname\@currname.\@currext-h@@k\endcsname}
\let\AtEndOfClass\AtEndOfPackage
\def\@cls@pkg{%
  \ifx\@currext\@clsextension
    document class%
  \else
    \ifx\@currext\@pkgextension
      package%
    \else
      file%
    \fi
  \fi}
\def\@unknownoptionerror{%
  \@latex@error
    {Unknown option `\CurrentOption' for \@cls@pkg\space`\@currname'}%
    {The option `\CurrentOption' was not declared in
     \@cls@pkg\space`\@currname', perhaps you\MessageBreak
      misspelled its name.
     Try typing \space <return>
     \space to proceed.}}
\def\@@unprocessedoptions{%
  \ifx\@currext\@pkgextension
    \protected@edef\@curroptions{\@ptionlist{\@currname.\@currext}}%
    \@for\CurrentOption:=\@curroptions\do{%
        \ifx\CurrentOption\@empty\else\@unknownoptionerror\fi}%
  \fi}
\def\@twoloadclasserror{%
  \@latex@error
    {Two \noexpand\LoadClass commands}%
    {You may only use one \noexpand\LoadClass in a class file}}
\let\flashtex@baseclass\@empty
\def\flashtex@declined#1[#2]#3{%
  \@pushfilename
  \xdef\@currname{#1}%
  \global\let\@currext#3%
  \@ifl@aded\@currext\@currname
    {\@onefilewithoptions@clashchk{#2}{}}%
    {\@pass@ptions\@currext{#2}{\@currname}%
     \global\expandafter
     \let\csname ver@\@currname.\@currext\endcsname\@empty
     \ifx\@currext\@clsextension\xdef\flashtex@baseclass{#1}\fi}%
  \@popfilename}
\def\flashtex@passthrough#1#2#3{%
  \edef\reserved@b{\@ptionlist{#2.#3}}%
  \ifx\reserved@b\@empty
    \edef\reserved@a{\noexpand\flashtex@emit{\noexpand#1{#2}}}%
  \else
    \edef\reserved@a{\noexpand\flashtex@emit{\noexpand#1[\reserved@b]{#2}}}%
  \fi
  \reserved@a}
\def\flashtex@classfallback#1#2{%
  \ifx\flashtex@baseclass\@empty
    \@latex@warning@no@line{Class `#2' has no \noexpand\LoadClass and
      loads no standard class; the page layout of `article' is used}%
    \xdef\flashtex@baseclass{article}%
    \edef\reserved@a{\noexpand\flashtex@emit{\noexpand#1[\@classoptionslist]{article}}}%
    \reserved@a
  \fi}
\makeatother
";

impl Engine {
    /// Let `\usepackage`/`\RequirePackage`/`\documentclass`/`\LoadClass`
    /// read project files: `reader(name, ext)` returns the text of
    /// `name.ext` or `None` to decline (see [`PackageReader`]). Without a
    /// reader every name is declined and the commands pass through, as they
    /// did before the package kernel existed. The reader travels with
    /// incremental checkpoints, like the font metrics.
    pub fn set_package_reader(&mut self, reader: PackageReader) {
        self.package_reader = Some(reader);
    }

    /// Every `.sty`/`.cls` file opened so far, in loading order.
    pub fn opened_package_files(&self) -> &[OpenedFile] {
        &self.opened_packages
    }

    /// `\usepackage[opts]{a,b}[version]` and its three siblings
    /// (`ltclass.dtx`'s `\@fileswithoptions`/`\@fileswith@ptions`/
    /// `\@fileswith@pti@ns`, latex.ltx 18699-18738): read the optional
    /// `[options]`, the `{names}` and the optional `[version]` without
    /// expansion, then queue one `\@onefilewithoptions` per name the host
    /// has a file for and one `\flashtex@declined` (records only) plus a
    /// pass-through emission per name it declines.
    ///
    /// Pass-through keeps the parser's input byte-for-byte: when every name
    /// is declined and none has options passed to it beforehand, the very
    /// tokens read here (the command included) are emitted again. Only a
    /// list that mixes loaded and declined names, or a name with
    /// `\PassOptionsToPackage`d options, is re-spelt one command per name
    /// (`\flashtex@passthrough`), with the passed options folded in the way
    /// `\@ptionlist` reports them.
    pub(crate) fn do_load_files(&mut self, tok: Token, kind: LoadKind) -> Step {
        // The command itself, renamed for the parser when it is one it does
        // not model (`\RequirePackage` is `\usepackage` to it).
        let command = Token::new(TokenKind::ControlSequence(kind.pass_through_name().into()), tok.span);
        // The command's own invocation origin (`None` when it was read from
        // a source text): what its pass-through tokens keep, so the host
        // places them exactly as it did before this kernel existed.
        let command_origin = self.last_origin;
        let mut taken: Vec<Pending> = Vec::new();
        // `\@ifnextchar[`: spaces before the bracket are skipped.
        let options = self.take_bracketed(&mut taken);
        let Some(names) = self.take_braced(&mut taken) else {
            // No `{names}`: hand everything back untouched, so the parser
            // reports the missing argument exactly as it did before.
            self.push_pending_as_read(taken);
            return Step::Emit(command);
        };
        let version = self.take_bracketed(&mut taken);
        let list = self.detokenize(&names.iter().map(|p| p.tok.clone()).collect::<Vec<_>>());
        let names: Vec<String> = list
            .split(',')
            .map(|name| name.split_whitespace().collect::<String>())
            .filter(|name| !name.is_empty())
            .collect();
        let ext = kind.extension();
        let found: Vec<bool> = names.iter().map(|name| self.package_file_exists(name, ext)).collect();
        let passed: Vec<bool> = names.iter().map(|name| self.st.scopes.is_defined(&format!("opt@{name}.{ext}"))).collect();
        let at = tok.span;
        // Kernel tokens are attributed to the loading command: a diagnostic
        // raised by the kernel code, and the file's `loaded_at`, point at it.
        let synth = |kind: TokenKind| Pending { tok: Token::new(kind, at), frozen: false, origin: Some(at) };
        let cs = |name: &str| synth(TokenKind::ControlSequence(name.into()));
        let group = |inner: &[Pending]| -> Vec<Pending> {
            let mut out = vec![synth(TokenKind::Char('{', CatCode::BeginGroup))];
            out.extend(inner.iter().cloned());
            out.push(synth(TokenKind::Char('}', CatCode::EndGroup)));
            out
        };
        let bracketed = |inner: &[Pending]| -> Vec<Pending> {
            let mut out = vec![synth(TokenKind::Char('[', CatCode::Other))];
            out.extend(group(inner));
            out.push(synth(TokenKind::Char(']', CatCode::Other)));
            out
        };
        let option_tokens: Vec<Pending> = options.clone().unwrap_or_default();
        let version_tokens: Vec<Pending> = version.clone().unwrap_or_default();
        let verbatim = found.iter().all(|f| !f) && passed.iter().all(|p| !p);
        let mut queue: Vec<Pending> = Vec::new();
        if kind.is_class() {
            // `\@fileswith@pti@ns`: the first class load fixes the global
            // option list every later `\ProcessOptions` consults.
            queue.push(cs("flashtex@setclassoptions"));
            queue.extend(group(&option_tokens));
        }
        for (i, name) in names.iter().enumerate() {
            let name_tokens: Vec<Pending> = name.chars().map(|c| synth(name_char(c))).collect();
            if found[i] {
                queue.push(cs("@onefilewithoptions"));
                queue.extend(group(&name_tokens));
                queue.extend(bracketed(&option_tokens));
                queue.extend(bracketed(&version_tokens));
                queue.push(cs(kind.extension_macro()));
            } else {
                queue.push(cs("flashtex@declined"));
                queue.extend(group(&name_tokens));
                queue.extend(bracketed(&option_tokens));
                queue.push(cs(kind.extension_macro()));
                if !verbatim {
                    queue.push(cs("flashtex@passthrough"));
                    queue.push(Pending { tok: command.clone(), frozen: false, origin: Some(at) });
                    queue.extend(group(&name_tokens));
                    queue.push(cs(kind.extension_macro()));
                }
            }
        }
        if verbatim {
            // Everything read, exactly as read, behind the command. The
            // group's own tokens carry the command's origin: the last one
            // read before the emission decides what the emitted tokens
            // are attributed to.
            let passed = |kind: TokenKind| Pending { tok: Token::new(kind, at), frozen: false, origin: command_origin };
            queue.push(passed(TokenKind::ControlSequence("flashtex@emit".into())));
            queue.push(passed(TokenKind::Char('{', CatCode::BeginGroup)));
            queue.push(Pending { tok: command, frozen: false, origin: command_origin });
            queue.extend(taken);
            queue.push(passed(TokenKind::Char('}', CatCode::EndGroup)));
        }
        if kind == LoadKind::DocumentClass {
            // After the class (and whatever it `\LoadClass`es) has been
            // read: a class file that named no standard class gets
            // `article`'s page model, with a warning saying so.
            for name in &names {
                queue.push(cs("flashtex@classfallback"));
                queue.push(Pending { tok: Token::new(TokenKind::ControlSequence("documentclass".into()), at), frozen: false, origin: Some(at) });
                queue.extend(group(&name.chars().map(|c| synth(name_char(c))).collect::<Vec<_>>()));
            }
        }
        self.push_pending_as_read(queue);
        Step::Continue
    }

    /// `[ ... ]` after optional spaces, unexpanded, `]` matched at brace
    /// depth 0 (as a `#1[#2]` delimited argument is). Every token read,
    /// spaces and brackets included, is appended to `taken`; the tokens
    /// between the brackets are returned.
    pub(crate) fn take_bracketed(&mut self, taken: &mut Vec<Pending>) -> Option<Vec<Pending>> {
        let mut spaces = Vec::new();
        loop {
            let Some(p) = self.next_raw() else {
                self.push_pending_as_read(spaces);
                return None;
            };
            match &p.tok.kind {
                TokenKind::Char(_, CatCode::Space) => spaces.push(p),
                TokenKind::Char('[', _) => {
                    taken.extend(spaces);
                    taken.push(p);
                    break;
                }
                _ => {
                    self.push_pending_as_read(vec![p]);
                    self.push_pending_as_read(spaces);
                    return None;
                }
            }
        }
        let mut inner = Vec::new();
        let mut depth = 0i32;
        while let Some(p) = self.next_raw() {
            match &p.tok.kind {
                TokenKind::Char(_, CatCode::BeginGroup) => depth += 1,
                TokenKind::Char(_, CatCode::EndGroup) => depth -= 1,
                TokenKind::Char(']', _) if depth <= 0 => {
                    taken.push(p);
                    return Some(inner);
                }
                _ => {}
            }
            inner.push(p.clone());
            taken.push(p);
        }
        Some(inner)
    }

    /// `{ ... }` after optional spaces, unexpanded and brace-balanced,
    /// recorded in `taken` like [`Engine::take_bracketed`].
    pub(crate) fn take_braced(&mut self, taken: &mut Vec<Pending>) -> Option<Vec<Pending>> {
        let mut spaces = Vec::new();
        loop {
            let Some(p) = self.next_raw() else {
                self.push_pending_as_read(spaces);
                return None;
            };
            match &p.tok.kind {
                TokenKind::Char(_, CatCode::Space) => spaces.push(p),
                TokenKind::Char(_, CatCode::BeginGroup) => {
                    taken.extend(spaces);
                    taken.push(p);
                    break;
                }
                _ => {
                    self.push_pending_as_read(vec![p]);
                    self.push_pending_as_read(spaces);
                    return None;
                }
            }
        }
        let mut inner = Vec::new();
        let mut depth = 1i32;
        while let Some(p) = self.next_raw() {
            match &p.tok.kind {
                TokenKind::Char(_, CatCode::BeginGroup) => depth += 1,
                TokenKind::Char(_, CatCode::EndGroup) => {
                    depth -= 1;
                    if depth == 0 {
                        taken.push(p);
                        return Some(inner);
                    }
                }
                _ => {}
            }
            inner.push(p.clone());
            taken.push(p);
        }
        Some(inner)
    }

    /// One of the four [`Declaration`]s: the kernel macro while a package
    /// or class file is being read (`\@currext` is `sty` or `cls`), the
    /// unchanged token for the host parser otherwise.
    pub(crate) fn do_declaration(&mut self, tok: Token, declaration: Declaration) -> Step {
        if self.in_package_file() {
            let name = format!("flashtex@{}", declaration.name());
            self.push_tokens(vec![Token::new(TokenKind::ControlSequence(name), tok.span)]);
            Step::Continue
        } else {
            Step::Emit(tok)
        }
    }

    /// Whether `\@currext` is non-empty: a `.sty`/`.cls` is being read.
    fn in_package_file(&self) -> bool {
        let mut meaning = self.st.scopes.meaning("@currext");
        while let crate::scopes::Meaning::Let(inner) = meaning {
            meaning = *inner;
        }
        match meaning {
            crate::scopes::Meaning::Macro(def) => !def.body.is_empty(),
            _ => false,
        }
    }

    fn package_file_exists(&self, name: &str, ext: &str) -> bool {
        self.package_reader.as_ref().is_some_and(|reader| reader(name, ext).is_some())
    }

    /// `\flashtex@inputfile{name}{ext}`: start reading `name.ext` from the
    /// host, exactly where `\@onefilewithoptions` calls
    /// `\InputIfFileExists` -- the rest of that macro's body waits below
    /// the file on the input stack and runs when the file ends (or
    /// `\endinput`s). Tokens of the file carry a source id of their own.
    pub(crate) fn do_input_package_file(&mut self, tok: &Token) -> Step {
        let name = self.read_name_arg();
        let ext = self.read_name_arg();
        let text = self.package_reader.as_ref().and_then(|reader| reader(&name, &ext));
        match text {
            Some(text) => {
                let id = self.st.next_source_id;
                self.st.next_source_id += 1;
                let loaded_at = self.last_origin.unwrap_or(tok.span);
                self.opened_packages.push(OpenedFile {
                    source_id: id,
                    name: format!("{name}.{ext}"),
                    loaded_at,
                    provides: None,
                    options: Vec::new(),
                    definitions: Vec::new(),
                });
                self.prune_exhausted();
                self.sources.push(Input::Text(Lexer::new(Rc::from(text), id)));
            }
            None => self.err(format!("LaTeX Error: File `{name}.{ext}' not found."), tok.span),
        }
        Step::Continue
    }

    /// `\flashtex@emit{tokens}`: hand the (unexpanded) group's tokens to
    /// the output ahead of everything else, as a pass-through. A token
    /// spelt by kernel code (a prelude span) is given the span of the
    /// first token in the group -- the loading command -- so the parser
    /// sees one contiguous source location; tokens read from a document
    /// keep their own spans.
    pub(crate) fn do_emit_pass_through(&mut self) -> Step {
        let toks = self.scan_braced_group(false);
        let at = toks.first().map(|t| t.span).unwrap_or(Span::synthetic());
        for t in toks.into_iter().rev() {
            let span = if self.is_prelude_span(t.span) || t.span.is_synthetic() { at } else { t.span };
            self.emit_queue.push(Token::new(t.kind, span));
        }
        Step::Continue
    }

    /// `\flashtex@latex@error{text}` / `\flashtex@latex@warning{text}`:
    /// the message is expanded (`\@currname`, `\CurrentOption` and the
    /// like resolve), printed as `\errmessage` would print it, and
    /// recorded with LaTeX's terminating period. The span is the loading
    /// command or the offending document token, per `Engine::err`.
    pub(crate) fn do_latex_message(&mut self, tok: &Token, error: bool) -> Step {
        let toks = self.scan_braced_group(true);
        let mut text = self.detokenize(&toks).split_whitespace().collect::<Vec<_>>().join(" ");
        if !text.ends_with('.') {
            text.push('.');
        }
        if error {
            self.err(text, tok.span);
        } else {
            self.warn(text, tok.span);
        }
        Step::Continue
    }
}

/// A character of a package/class name as `\@onefilewithoptions` stores it
/// in `\@currname` (`\string@makeletter`, latex.ltx 18747): letters stay
/// letters, so `\ProvidesPackage{name}` compares equal token by token.
fn name_char(c: char) -> TokenKind {
    TokenKind::Char(c, if c.is_ascii_alphabetic() { CatCode::Letter } else { CatCode::Other })
}
