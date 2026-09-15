// name: KeyboardShortcut.cs
// purpose: Framework-agnostic keyboard shortcut spelling for CommandRegistry
//   entries. Deliberately not a WinUI KeyboardAccelerator: this library has no
//   WinUI/XAML dependency, so a future UI layer converts one of these into
//   whatever accelerator type its framework wants.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Shell;

/// <summary>
/// Modifier keys held with a shortcut's primary key, matching Windows
/// convention (Ctrl/Alt in place of the Mac source's Cmd/Option).
/// </summary>
[Flags]
public enum ShortcutModifiers
{
    None = 0,
    Ctrl = 1 << 0,
    Alt = 1 << 1,
    Shift = 1 << 2,
    Win = 1 << 3,
}

/// <summary>
/// One keyboard shortcut: a modifier combination plus a primary key spelled as
/// a short display string ("P", "]", "+"). Equality and <see cref="ToString"/>
/// are used both for display (menu/toolbar tooltips) and as one of the fields
/// <see cref="CommandRegistry"/> searches over, so the display spelling must
/// stay stable.
/// </summary>
public readonly record struct KeyboardShortcut(ShortcutModifiers Modifiers, string Key)
{
    /// <summary>No shortcut assigned (a command reachable only from a menu or the palette).</summary>
    public static readonly KeyboardShortcut None = new(ShortcutModifiers.None, string.Empty);

    /// <summary>Whether this shortcut has a key at all (i.e. is not <see cref="None"/>).</summary>
    public bool HasKey => Key.Length > 0;

    /// <summary>"Ctrl+Shift+P" style spelling, in a fixed modifier order.</summary>
    public override string ToString()
    {
        if (!HasKey)
        {
            return string.Empty;
        }

        var parts = new List<string>(capacity: 4);
        if (Modifiers.HasFlag(ShortcutModifiers.Ctrl))
        {
            parts.Add("Ctrl");
        }

        if (Modifiers.HasFlag(ShortcutModifiers.Alt))
        {
            parts.Add("Alt");
        }

        if (Modifiers.HasFlag(ShortcutModifiers.Shift))
        {
            parts.Add("Shift");
        }

        if (Modifiers.HasFlag(ShortcutModifiers.Win))
        {
            parts.Add("Win");
        }

        parts.Add(Key);
        return string.Join('+', parts);
    }
}
