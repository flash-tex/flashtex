// name: CompletionVocabulary.cs
// purpose: Decodes the compiler's machine-readable command inventory
//   (Resources/supported-latex.json, schema flashtex-supported-latex/1,
//   embedded as a build resource) into the completion vocabulary: one Entry
//   per renderable command, in the same offer order as
//   apps/mac/Sources/FlashTeXMac/Completion.swift's `Vocabulary.buildEntries`.
//   Ported unit for unit from that file's pure `Vocabulary` enum (derived
//   tables, hand-written overrides and entry-ordering rules); JSON decoding
//   uses System.Text.Json instead of Swift's JSONDecoder.
//
//   Scope note: only the vocabulary table is ported here -- not
//   `Vocabulary`'s AppKit/Bundle resource lookup (`inventoryCandidates` /
//   `loadInventoryData`, which searches an app bundle on disk for the JSON
//   at multiple candidate paths). This port loads the JSON from an embedded
//   assembly resource instead (see Resources\supported-latex.json and the
//   <EmbeddedResource> item in FlashTeX.Editor.csproj); refresh instructions
//   are in that csproj entry, mirroring apps/mac/scripts/sync-supported-latex.sh.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Reflection;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace FlashTeX.Editor;

/// <summary>Whether a command/environment is accepted in prose or math mode.</summary>
public enum CompletionMode
{
    Text,
    Math,
}

/// <summary>Which compiler table accepts the command (the inventory's `origin`).</summary>
public enum CompletionOrigin
{
    TextDispatch,
    TextStyle,
    TextSize,
    MathStructure,
    MathSymbol,
    MathOperator,
    ControlSymbol,

    /// <summary>Executed by the expansion pass: a TeX/LaTeX primitive or kernel
    /// macro (`\newcounter`, `\setcounter`, `\providecommand`, ...).</summary>
    Expansion,
}

/// <summary>One command's popup label, snippet argument shape and one-line docs.</summary>
public sealed record CompletionEntry(
    string Name,
    string Arguments,
    string Description,
    CompletionMode Mode,
    CompletionOrigin Origin,
    string? Glyph,
    string? MathDescription)
{
    /// <summary>Popup label, e.g. `\section{...}` or `\alpha`.</summary>
    public string Label => "\\" + Name + Arguments;

    /// <summary>Detail text shown beside the label: the mode-appropriate
    /// description, with the other mode's behaviour appended when known.</summary>
    public string Detail => Mode == CompletionMode.Math
        ? "math · " + Description
        : MathDescription is { } math ? Description + " · in math: " + math : Description;
}

/// <summary>The compiler's documented command set and environment list, decoded
/// once from the bundled inventory resource.</summary>
public static class CompletionVocabulary
{
    private const string ResourceName = "FlashTeX.Editor.Resources.supported-latex.json";
    private const string ExpectedSchema = "flashtex-supported-latex/1";
    private const string GenericDescription = "supported by this compiler";
    private const char Backslash = '\\';

    /// <summary>Every rendered command of the inventory, once each, in offer
    /// order: text-mode commands (dispatch, style and size, in file order),
    /// then math structures, operators and symbols (each in file order).</summary>
    public static IReadOnlyList<CompletionEntry> Entries { get; }

    public static IReadOnlyDictionary<string, CompletionEntry> ByName { get; }

    public static IReadOnlyList<string> Names { get; }

    /// <summary>Text/display environments and the math grids the compiler accepts, in file order.</summary>
    public static IReadOnlyList<string> Environments { get; }

    static CompletionVocabulary()
    {
        Inventory inventory = LoadInventory();
        Entries = BuildEntries(inventory);
        ByName = Entries.ToDictionary(static e => e.Name, StringComparer.Ordinal);
        Names = Entries.Select(static e => e.Name).ToArray();
        Environments = inventory.Environments.Select(static e => e.Name).ToArray();
    }

    /// <summary>Entry for a name outside the table (a caller-supplied list, e.g.
    /// a command declared elsewhere in the document).</summary>
    public static CompletionEntry Generic(string name) => new(
        Name: name, Arguments: "", Description: GenericDescription,
        Mode: CompletionMode.Text, Origin: CompletionOrigin.TextDispatch, Glyph: null, MathDescription: null);

    private static Inventory LoadInventory()
    {
        Assembly assembly = typeof(CompletionVocabulary).Assembly;
        using Stream stream = assembly.GetManifestResourceStream(ResourceName)
            ?? throw new InvalidOperationException($"embedded resource '{ResourceName}' not found");
        Inventory? inventory = JsonSerializer.Deserialize<Inventory>(stream, JsonOptions);
        if (inventory is null)
        {
            throw new InvalidOperationException("supported-latex.json deserialized to null");
        }
        if (inventory.Schema != ExpectedSchema)
        {
            throw new InvalidOperationException($"unexpected schema '{inventory.Schema}'; expected '{ExpectedSchema}'");
        }
        return inventory;
    }

    private static List<CompletionEntry> BuildEntries(Inventory inventory)
    {
        List<InventoryCommand> rendered = inventory.Commands.Where(static c => c.Renders).ToList();
        Dictionary<string, string> mathDescriptions = new(StringComparer.Ordinal);
        foreach (InventoryCommand c in rendered.Where(static c => c.Mode == CompletionMode.Math))
        {
            mathDescriptions.TryAdd(c.Name, c.Description);
        }

        HashSet<string> seen = new(StringComparer.Ordinal);
        List<CompletionEntry> entries = new();
        void Add(InventoryCommand c, string? mathDescription = null)
        {
            if (!seen.Add(c.Name))
            {
                return;
            }
            entries.Add(new CompletionEntry(c.Name, c.Arguments, c.Description, c.Mode, c.Origin, c.Glyph, mathDescription));
        }

        foreach (InventoryCommand c in rendered.Where(static c => c.Mode == CompletionMode.Text && c.Origin != CompletionOrigin.ControlSymbol))
        {
            Add(c, mathDescriptions.GetValueOrDefault(c.Name));
        }
        foreach (InventoryCommand c in rendered.Where(static c => c.Origin == CompletionOrigin.ControlSymbol && c.Name == Backslash.ToString()))
        {
            Add(c);
        }
        foreach (CompletionOrigin origin in new[] { CompletionOrigin.MathStructure, CompletionOrigin.MathOperator, CompletionOrigin.MathSymbol })
        {
            foreach (InventoryCommand c in rendered.Where(c => c.Origin == origin))
            {
                Add(c);
            }
        }
        return entries;
    }

    // MARK: JSON shape (`flashtex-supported-latex/1`; only the fields the editor uses)

    private sealed record InventoryCommand(
        string Name, CompletionMode Mode, CompletionOrigin Origin, string Arguments, string Description, string? Glyph, bool Renders);

    private sealed record InventoryEnvironment(string Name, CompletionMode Mode, string Description);

    private sealed record Inventory(
        string Schema,
        [property: JsonPropertyName("compiler_version")] string CompilerVersion,
        IReadOnlyList<InventoryCommand> Commands,
        IReadOnlyList<InventoryEnvironment> Environments);

    private static readonly JsonSerializerOptions JsonOptions = CreateJsonOptions();

    private static JsonSerializerOptions CreateJsonOptions()
    {
        var options = new JsonSerializerOptions(JsonSerializerDefaults.Web);
        options.Converters.Add(new JsonStringEnumConverter(new SnakeCaseLowerNamingPolicy()));
        return options;
    }

    /// <summary>Converts a PascalCase enum member name to the inventory's
    /// snake_case spelling (`TextDispatch` -&gt; `text_dispatch`).</summary>
    private sealed class SnakeCaseLowerNamingPolicy : JsonNamingPolicy
    {
        public override string ConvertName(string name)
        {
            var builder = new StringBuilder(name.Length + 4);
            for (int i = 0; i < name.Length; i++)
            {
                char c = name[i];
                if (char.IsUpper(c))
                {
                    if (i > 0)
                    {
                        builder.Append('_');
                    }
                    builder.Append(char.ToLowerInvariant(c));
                }
                else
                {
                    builder.Append(c);
                }
            }
            return builder.ToString();
        }
    }
}
