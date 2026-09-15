// name: SignatureHelpTests.cs
// purpose: Unit tests for FlashTeX.Editor.SignatureHelp, ported behavior from
//   apps/mac/Sources/FlashTeXMac/SignatureHelp.swift's pure `Info.info` model
//   (caret -> enclosing command + active argument index) and `Info.display`.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Editor.Tests;

public class SignatureHelpTests
{
    // MARK: groups(of:)

    [Fact]
    public void GroupsOfSplitsBracedAndBracketedArguments()
    {
        Assert.Equal(new[] { "{num}", "{den}" }, SignatureHelp.GroupsOf("{num}{den}"));
        Assert.Equal(new[] { "[options]", "{class}" }, SignatureHelp.GroupsOf("[options]{class}"));
    }

    [Fact]
    public void GroupsOfEmptyArgumentsIsEmpty()
    {
        Assert.Empty(SignatureHelp.GroupsOf(""));
    }

    // MARK: info(text, caret): zero, one, two-argument and optional-bracket commands

    [Fact]
    public void ZeroArgumentCommandHasNoEnclosingArgument()
    {
        // \par takes no arguments, so no `{`/`[` follows it to sit inside.
        Assert.Null(SignatureHelp.Info("\\par", 4));
    }

    [Fact]
    public void OneArgumentCommandFirstArgument()
    {
        string text = "\\section{intro";
        SignatureHelpInfo? info = SignatureHelp.Info(text, text.Length);
        Assert.NotNull(info);
        Assert.Equal("section", info!.Value.Command);
        Assert.Equal(0, info.Value.ActiveArgument);
        Assert.Equal("{...}", info.Value.Arguments);
    }

    [Fact]
    public void TwoArgumentCommandFirstArgument()
    {
        string text = "\\frac{a";
        SignatureHelpInfo? info = SignatureHelp.Info(text, text.Length);
        Assert.NotNull(info);
        Assert.Equal("frac", info!.Value.Command);
        Assert.Equal(0, info.Value.ActiveArgument);
    }

    [Fact]
    public void TwoArgumentCommandSecondArgument()
    {
        string text = "\\frac{a}{b";
        SignatureHelpInfo? info = SignatureHelp.Info(text, text.Length);
        Assert.NotNull(info);
        Assert.Equal("frac", info!.Value.Command);
        Assert.Equal(1, info.Value.ActiveArgument);
    }

    [Fact]
    public void OptionalBracketArgumentIsRecognisedAsArgumentZero()
    {
        // \documentclass[options]{class}: caret inside the optional `[...]`.
        string text = "\\documentclass[12pt";
        SignatureHelpInfo? info = SignatureHelp.Info(text, text.Length);
        Assert.NotNull(info);
        Assert.Equal("documentclass", info!.Value.Command);
        Assert.Equal(0, info.Value.ActiveArgument);
    }

    [Fact]
    public void OptionalBracketArgumentThenBracedArgumentCountsTheClosedOptional()
    {
        string text = "\\documentclass[12pt]{art";
        SignatureHelpInfo? info = SignatureHelp.Info(text, text.Length);
        Assert.NotNull(info);
        Assert.Equal("documentclass", info!.Value.Command);
        Assert.Equal(1, info.Value.ActiveArgument);
    }

    [Fact]
    public void CaretOutsideAnyArgumentIsNull()
    {
        string text = "\\section{intro} plain text";
        Assert.Null(SignatureHelp.Info(text, text.Length));
    }

    [Fact]
    public void CaretInsideACommentIsNull()
    {
        string text = "% \\section{intro";
        Assert.Null(SignatureHelp.Info(text, text.Length));
    }

    [Fact]
    public void UnknownCommandNameIsNull()
    {
        string text = "\\notarealcommand{x";
        Assert.Null(SignatureHelp.Info(text, text.Length));
    }

    // MARK: display

    [Fact]
    public void DisplayHighlightsTheActiveArgumentGroup()
    {
        string text = "\\frac{a}{b";
        SignatureHelpInfo info = SignatureHelp.Info(text, text.Length)!.Value;
        (string display, Utf16Range? active) = info.Display;
        Assert.Equal("\\frac{num}{den}", display);
        Assert.NotNull(active);
        // The second group, "{den}", starts right after "\frac{num}".
        Assert.Equal("\\frac{num}".Length, active!.Value.Location);
        Assert.Equal("{den}".Length, active.Value.Length);
    }

    [Fact]
    public void DisplayForAZeroArgumentCommandIsJustTheCommand()
    {
        SignatureHelpInfo info = new("par", "", 0, "ends the paragraph", 0);
        (string display, Utf16Range? active) = info.Display;
        Assert.Equal("\\par", display);
        Assert.Null(active);
    }
}
