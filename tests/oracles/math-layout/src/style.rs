//! Math styles and the style-change rules of TeXbook chapter 17 / Appendix G.

use crate::metrics::SizeClass;

/// The four base styles: display, text, script, scriptscript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StyleLevel {
    Display,
    Text,
    Script,
    ScriptScript,
}

/// A math style: a base level plus the cramped flag (D, D', T, T', S, S', SS, SS').
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Style {
    pub level: StyleLevel,
    pub cramped: bool,
}

impl Style {
    pub const DISPLAY: Style = Style {
        level: StyleLevel::Display,
        cramped: false,
    };
    pub const TEXT: Style = Style {
        level: StyleLevel::Text,
        cramped: false,
    };
    pub const SCRIPT: Style = Style {
        level: StyleLevel::Script,
        cramped: false,
    };
    pub const SCRIPT_SCRIPT: Style = Style {
        level: StyleLevel::ScriptScript,
        cramped: false,
    };

    pub fn cramped(self) -> Style {
        Style {
            cramped: true,
            ..self
        }
    }

    pub fn is_display(self) -> bool {
        self.level == StyleLevel::Display
    }

    /// Whether the TeXbook spacing table's parenthesised entries apply
    /// (only in display and text styles).
    pub fn is_uncompressed(self) -> bool {
        matches!(self.level, StyleLevel::Display | StyleLevel::Text)
    }

    pub fn size_class(self) -> SizeClass {
        match self.level {
            StyleLevel::Display | StyleLevel::Text => SizeClass::Text,
            StyleLevel::Script => SizeClass::Script,
            StyleLevel::ScriptScript => SizeClass::ScriptScript,
        }
    }

    /// Superscript style: D,T → S; S,SS → SS; cramping is preserved.
    pub fn sup(self) -> Style {
        let level = match self.level {
            StyleLevel::Display | StyleLevel::Text => StyleLevel::Script,
            StyleLevel::Script | StyleLevel::ScriptScript => StyleLevel::ScriptScript,
        };
        Style {
            level,
            cramped: self.cramped,
        }
    }

    /// Subscript style: the cramped superscript style.
    pub fn sub(self) -> Style {
        self.sup().cramped()
    }

    /// Numerator style: D → T, T → S, S,SS → SS; cramping is preserved.
    pub fn num(self) -> Style {
        let level = match self.level {
            StyleLevel::Display => StyleLevel::Text,
            StyleLevel::Text => StyleLevel::Script,
            StyleLevel::Script | StyleLevel::ScriptScript => StyleLevel::ScriptScript,
        };
        Style {
            level,
            cramped: self.cramped,
        }
    }

    /// Denominator style: the cramped numerator style.
    pub fn denom(self) -> Style {
        self.num().cramped()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_transitions_follow_the_texbook() {
        assert_eq!(Style::DISPLAY.sup(), Style::SCRIPT);
        assert_eq!(Style::DISPLAY.sub(), Style::SCRIPT.cramped());
        assert_eq!(Style::TEXT.num(), Style::SCRIPT);
        assert_eq!(Style::DISPLAY.num(), Style::TEXT);
        assert_eq!(Style::DISPLAY.denom(), Style::TEXT.cramped());
        assert_eq!(Style::SCRIPT.sup(), Style::SCRIPT_SCRIPT);
        assert_eq!(Style::SCRIPT_SCRIPT.sub(), Style::SCRIPT_SCRIPT.cramped());
        assert_eq!(Style::TEXT.cramped().sup(), Style::SCRIPT.cramped());
    }
}
