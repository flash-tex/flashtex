// name: KeyboardShortcutTranslator.cs
// purpose: Translates a framework-agnostic FlashTeX.Shell.KeyboardShortcut
//   (modifier flags + a short display key string, e.g. "Ctrl+Shift+P") into
//   WinUI3's Windows.System.VirtualKeyModifiers/VirtualKey pair for a
//   Microsoft.UI.Xaml.Input.KeyboardAccelerator. Kept in FlashTeX.App (not
//   FlashTeX.Shell) because FlashTeX.Shell has no WinUI dependency by design;
//   this is the one small seam that needs one.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;
using Windows.System;

namespace FlashTeX.App;

/// <summary>Converts a <see cref="KeyboardShortcut"/> into WinUI3 accelerator types.</summary>
internal static class KeyboardShortcutTranslator
{
    /// <summary>Display keys with no direct <see cref="VirtualKey"/> enum member of the same name.</summary>
    private static readonly IReadOnlyDictionary<string, VirtualKey> SymbolKeys = new Dictionary<string, VirtualKey>(StringComparer.Ordinal)
    {
        ["+"] = VirtualKey.Add,
        ["-"] = VirtualKey.Subtract,
        ["["] = (VirtualKey)219, // VK_OEM_4
        ["]"] = (VirtualKey)221, // VK_OEM_6
    };

    /// <summary>
    /// Attempts to translate <paramref name="shortcut"/>. Returns false for
    /// <see cref="KeyboardShortcut.None"/> (no key assigned) or a key this
    /// translator does not recognize, so the caller can simply skip adding an
    /// accelerator rather than throwing for a command with no shortcut.
    /// </summary>
    public static bool TryTranslate(KeyboardShortcut shortcut, out VirtualKeyModifiers modifiers, out VirtualKey key)
    {
        modifiers = VirtualKeyModifiers.None;
        key = VirtualKey.None;
        if (!shortcut.HasKey || !TryTranslateKey(shortcut.Key, out key))
        {
            return false;
        }

        modifiers = TranslateModifiers(shortcut.Modifiers);
        return true;
    }

    private static VirtualKeyModifiers TranslateModifiers(ShortcutModifiers modifiers)
    {
        var result = VirtualKeyModifiers.None;
        if (modifiers.HasFlag(ShortcutModifiers.Ctrl))
        {
            result |= VirtualKeyModifiers.Control;
        }
        if (modifiers.HasFlag(ShortcutModifiers.Alt))
        {
            result |= VirtualKeyModifiers.Menu;
        }
        if (modifiers.HasFlag(ShortcutModifiers.Shift))
        {
            result |= VirtualKeyModifiers.Shift;
        }
        if (modifiers.HasFlag(ShortcutModifiers.Win))
        {
            result |= VirtualKeyModifiers.Windows;
        }
        return result;
    }

    private static bool TryTranslateKey(string displayKey, out VirtualKey key)
    {
        if (SymbolKeys.TryGetValue(displayKey, out key))
        {
            return true;
        }
        if (displayKey.Length == 1 && char.IsAsciiDigit(displayKey[0]))
        {
            key = VirtualKey.Number0 + (displayKey[0] - '0');
            return true;
        }
        if (displayKey.Length == 1 && char.IsAsciiLetterUpper(displayKey[0]))
        {
            key = Enum.Parse<VirtualKey>(displayKey);
            return true;
        }
        key = VirtualKey.None;
        return false;
    }
}
