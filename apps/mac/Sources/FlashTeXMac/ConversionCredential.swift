import Foundation
import Security

/// The capture-conversion provider the Mac hands to `flashtex-bridge` for
/// `capture_convert` (drawing/photo → LaTeX). This is the ONLY model-backed
/// feature the editor ships; everything else stays unopinionated
/// (docs/extensibility.md). Adding a provider means adding a case here and
/// teaching the bridge its flag; the app never talks to a provider itself.
enum ConversionProvider: String, CaseIterable, Equatable, Identifiable {
    /// No conversion: the bridge runs without a provider and `capture_convert`
    /// is refused with `provider_disabled`.
    case none
    /// xAI's vision LLM through the bridge's `--enable-grok` path.
    case xai

    var id: String { rawValue }

    /// The Preferences picker label.
    var label: String {
        switch self {
        case .none: return "None (conversion disabled)"
        case .xai: return "xAI (LLM)"
        }
    }

    /// Short display name for status text.
    var displayName: String {
        switch self {
        case .none: return "none"
        case .xai: return "xAI"
        }
    }

    /// Whether the provider needs an API key.
    var needsCredential: Bool { self != .none }

    /// Keychain service holding this provider's key
    /// (`tech.jay3332.flashtex.ai.<provider>`).
    var keychainService: String { ConversionCredential.keychainServicePrefix + rawValue }

    /// The bridge flag that switches this provider on (nil: no provider).
    var bridgeFlag: String? {
        switch self {
        case .none: return nil
        case .xai: return "--enable-grok"
        }
    }

    /// The environment variable the bridge reads the key from for this provider.
    var bridgeKeyVariable: String? {
        switch self {
        case .none: return nil
        case .xai: return "XAI_API_KEY"
        }
    }

    /// Provider-specific environment variables consulted after the generic
    /// `FLASHTEX_AI_API_KEY` (so an existing `XAI_API_KEY` keeps working).
    var legacyEnvironmentNames: [String] {
        switch self {
        case .none: return []
        case .xai: return ["XAI_API_KEY", "FLASHTEX_GROK_API_KEY"]
        }
    }

    /// Default model id. Live evidence 2026-09-12
    /// (docs/evidence/grok-live-20260912T191209Z): the fast non-reasoning xAI
    /// model converted a 900×260 handwriting PNG in 2.8 s and the result
    /// compiled with zero diagnostics; the reasoning model hit the bridge's
    /// 90 s timeout.
    var defaultModel: String {
        switch self {
        case .none: return ""
        case .xai: return "grok-4.20-0309-non-reasoning"
        }
    }

    /// Ids the Preferences picker offers (any valid id can still be typed).
    var selectableModels: [String] {
        switch self {
        case .none: return []
        case .xai: return ["grok-4.20-0309-non-reasoning", "grok-4.6"]
        }
    }

    /// Migration: the pre-provider Keychain item (`tech.jay3332.flashtex.xai`,
    /// accounts `xai` / `XAI_API_KEY`) is read when the provider's own item is
    /// absent and moved to the new service on first use.
    var legacyKeychainService: String? { self == .xai ? ConversionCredential.legacyXAIKeychainService : nil }
}

/// The Mac's conversion-provider credential adapter. The Rust bridge never
/// reads the Keychain: `flashtex-bridge --enable-grok` expects the key in its
/// environment (`XAI_API_KEY`), supplied by this adapter.
///
/// Source order for the selected provider: the Keychain generic-password item
/// (service `tech.jay3332.flashtex.ai.<provider>`, account `key`; for xAI also
/// the legacy `tech.jay3332.flashtex.xai` item), then the environment
/// (`FLASHTEX_AI_API_KEY`, then the provider's own names), else none.
/// `FLASHTEX_KEYCHAIN_OFF=1` skips the Keychain (environment-only mode, used by
/// tests and headless runs). The key is never logged, never placed in argv or
/// JSON, and never shown: every text this type produces says only
/// "present (source)" or "absent".
///
/// To supply a key without the Preferences window:
/// `security add-generic-password -U -s tech.jay3332.flashtex.ai.xai -a key -w '<key>'`
/// (`-U` updates an existing item); remove it with
/// `security delete-generic-password -s tech.jay3332.flashtex.ai.xai -a key`.
enum ConversionCredential {
    static let keychainServicePrefix = "tech.jay3332.flashtex.ai."
    static let keychainAccount = "key"
    /// The pre-provider item written by older builds (migrated on read).
    static let legacyXAIKeychainService = "tech.jay3332.flashtex.xai"
    static let legacyXAIKeychainAccounts = ["xai", "XAI_API_KEY"]
    /// The provider-neutral environment name, consulted first.
    static let environmentName = "FLASHTEX_AI_API_KEY"
    static let keychainOffVariable = "FLASHTEX_KEYCHAIN_OFF"
    static let providerVariable = "FLASHTEX_CONVERSION_PROVIDER"
    static let modelVariable = "FLASHTEX_CONVERSION_MODEL"
    /// Older names still honoured for the model (after `modelVariable`).
    static let legacyModelVariables = ["FLASHTEX_GROK_CAPTURE_MODEL", "FLASHTEX_GROK_MODEL"]
    /// The bridge's model variable (`--enable-grok` reads this one).
    static let bridgeModelVariable = "FLASHTEX_GROK_MODEL"
    /// The bridge's own limit (`GrokClient::new`).
    static let maxKeyBytes = 8192

    enum Source: String, Equatable {
        case keychain = "Keychain"
        case environment = "environment"
    }

    /// A resolved key. `description` (and every status text derived from it)
    /// reports presence and source only; the key itself is only ever handed
    /// to a child process environment by `inject(into:as:)`.
    struct Resolution: Equatable, CustomStringConvertible, CustomDebugStringConvertible {
        private let key: String
        let source: Source
        /// Which environment variable supplied it (nil for the Keychain).
        let variable: String?

        fileprivate init(key: String, source: Source, variable: String?) {
            self.key = key; self.source = source; self.variable = variable
        }

        var description: String { "key present (\(source.rawValue)\(variable.map { ": \($0)" } ?? ""))" }
        var debugDescription: String { description }
        /// Introspection for tests: the key's byte count, never its bytes.
        var keyByteCount: Int { key.utf8.count }

        /// Adds the key to `environment` under `name` (the only way the key leaves this type).
        func inject(into environment: inout [String: String], as name: String) {
            environment[name] = key
        }
    }

    /// Rejects what the bridge would reject, so a bad key fails here with a
    /// clear message rather than as a child exit.
    static func validate(_ raw: String) -> Result<String, ValidationError> {
        let key = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if key.isEmpty { return .failure(.empty) }
        if key.utf8.count > maxKeyBytes { return .failure(.tooLong) }
        if key.unicodeScalars.contains(where: { $0.value < 0x20 || $0.value == 0x7f }) { return .failure(.controlCharacters) }
        return .success(key)
    }

    enum ValidationError: Error, Equatable {
        case empty, tooLong, controlCharacters
        var text: String {
            switch self {
            case .empty: return "the key is empty"
            case .tooLong: return "the key exceeds \(ConversionCredential.maxKeyBytes) bytes"
            case .controlCharacters: return "the key contains control characters"
            }
        }
    }

    // MARK: provider and model selection

    /// The selected provider: `FLASHTEX_CONVERSION_PROVIDER` (a
    /// `ConversionProvider` raw value) wins over the preference.
    static func provider(environment: [String: String] = ProcessInfo.processInfo.environment,
                         preferences: ConversionPreferences = .shared) -> ConversionProvider {
        if let raw = environment[providerVariable]?.lowercased(), let p = ConversionProvider(rawValue: raw) { return p }
        return preferences.provider
    }

    /// Model id for the bridge: `FLASHTEX_CONVERSION_MODEL` (then the legacy
    /// names), else the preference, else the provider's default. Invalid
    /// values (the bridge's rule: ASCII alphanumerics plus `-._`, at most 128
    /// bytes) fall back to the default.
    static func model(for provider: ConversionProvider,
                      environment: [String: String] = ProcessInfo.processInfo.environment,
                      preferences: ConversionPreferences = .shared) -> String {
        let candidates = [environment[modelVariable]] + legacyModelVariables.map { environment[$0] } + [preferences.model(for: provider)]
        for candidate in candidates {
            if let candidate, isValidModel(candidate) { return candidate }
        }
        return provider.defaultModel
    }

    static func isValidModel(_ model: String) -> Bool {
        !model.isEmpty && model.utf8.count <= 128
            && model.utf8.allSatisfy { ($0 >= 0x30 && $0 <= 0x39) || ($0 >= 0x41 && $0 <= 0x5a) || ($0 >= 0x61 && $0 <= 0x7a) || $0 == 0x2d || $0 == 0x2e || $0 == 0x5f }
    }

    // MARK: resolution

    /// Keychain (unless `FLASHTEX_KEYCHAIN_OFF=1`), then the environment names
    /// in order. A Keychain error other than "no item" is reported through
    /// `keychainProblem` on the returned status, never thrown at callers.
    static func resolve(for provider: ConversionProvider,
                        environment: [String: String] = ProcessInfo.processInfo.environment,
                        keychain: any ConversionKeychainStore = SecItemKeychain.shared) -> Resolution? {
        status(for: provider, environment: environment, keychain: keychain).resolution
    }

    /// What the Preferences window and the capture notes show.
    struct Status: Equatable {
        var provider: ConversionProvider
        var resolution: Resolution?
        var keychainSkipped: Bool
        /// A Keychain read failure (OSStatus text), when one occurred.
        var keychainProblem: String?

        var text: String {
            guard provider.needsCredential else { return "conversion disabled: no provider selected" }
            let label = "\(provider.displayName) API key"
            if let resolution { return "\(label): \(resolution.description)" }
            var where_ = keychainSkipped ? "Keychain skipped (\(ConversionCredential.keychainOffVariable)=1)" : "no Keychain item \(provider.keychainService)"
            if let keychainProblem { where_ += " (\(keychainProblem))" }
            let names = ([ConversionCredential.environmentName] + provider.legacyEnvironmentNames).joined(separator: " / ")
            return "\(label): absent — \(where_); \(names) unset"
        }
    }

    /// Environment names consulted for `provider`, in order.
    static func environmentNames(for provider: ConversionProvider) -> [String] {
        [environmentName] + provider.legacyEnvironmentNames
    }

    static func status(for provider: ConversionProvider,
                       environment: [String: String] = ProcessInfo.processInfo.environment,
                       keychain: any ConversionKeychainStore = SecItemKeychain.shared) -> Status {
        let skipped = environment[keychainOffVariable] == "1"
        guard provider.needsCredential else { return Status(provider: provider, resolution: nil, keychainSkipped: skipped, keychainProblem: nil) }
        var problem: String?
        if !skipped {
            switch keychain.read(service: provider.keychainService, account: keychainAccount) {
            case .success(let stored?):
                if case .success(let key) = validate(stored) {
                    return Status(provider: provider, resolution: Resolution(key: key, source: .keychain, variable: nil), keychainSkipped: false, keychainProblem: nil)
                }
                problem = "stored item is not a usable key"
            case .success(nil):
                switch migrateLegacyItem(for: provider, keychain: keychain) {
                case .migrated(let key):
                    return Status(provider: provider, resolution: Resolution(key: key, source: .keychain, variable: nil), keychainSkipped: false, keychainProblem: nil)
                case .problem(let why): problem = why
                case .nothing: break
                }
            case .failure(let why): problem = why.text
            }
        }
        for name in environmentNames(for: provider) {
            if let raw = environment[name], case .success(let key) = validate(raw) {
                return Status(provider: provider, resolution: Resolution(key: key, source: .environment, variable: name), keychainSkipped: skipped, keychainProblem: problem)
            }
        }
        return Status(provider: provider, resolution: nil, keychainSkipped: skipped, keychainProblem: problem)
    }

    private enum Migration { case migrated(String), problem(String), nothing }

    /// Reads the pre-provider item, writes it under the provider's service and
    /// deletes the old accounts. `.nothing` when there is nothing to migrate.
    private static func migrateLegacyItem(for provider: ConversionProvider,
                                          keychain: any ConversionKeychainStore) -> Migration {
        guard let legacyService = provider.legacyKeychainService else { return .nothing }
        for account in legacyXAIKeychainAccounts {
            switch keychain.read(service: legacyService, account: account) {
            case .success(let stored?):
                guard case .success(let key) = validate(stored) else { return .problem("legacy item (account \(account)) is not a usable key") }
                if case .failure(let e) = keychain.write(key, service: provider.keychainService, account: keychainAccount) {
                    return .problem("could not migrate the legacy item: \(e.text)")
                }
                for old in legacyXAIKeychainAccounts { _ = keychain.delete(service: legacyService, account: old) }
                return .migrated(key)
            case .success(nil): continue
            case .failure(let e): return .problem(e.text)
            }
        }
        return .nothing
    }

    // MARK: storing (Preferences)

    /// Stores (or replaces) the provider's Keychain item. The Preferences
    /// window is the only caller; the key comes straight from its secure field.
    static func store(_ raw: String, for provider: ConversionProvider,
                      keychain: any ConversionKeychainStore = SecItemKeychain.shared) -> Result<Void, StoreError> {
        guard provider.needsCredential else { return .failure(.invalid(.empty)) }
        switch validate(raw) {
        case .failure(let e): return .failure(.invalid(e))
        case .success(let key):
            return keychain.write(key, service: provider.keychainService, account: keychainAccount).mapError { .keychain($0) }
        }
    }

    /// Deletes the provider's item (and, for xAI, the legacy accounts).
    static func remove(for provider: ConversionProvider,
                       keychain: any ConversionKeychainStore = SecItemKeychain.shared) -> Result<Void, StoreError> {
        if case .failure(let e) = keychain.delete(service: provider.keychainService, account: keychainAccount) { return .failure(.keychain(e)) }
        if let legacy = provider.legacyKeychainService {
            for account in legacyXAIKeychainAccounts {
                if case .failure(let e) = keychain.delete(service: legacy, account: account) { return .failure(.keychain(e)) }
            }
        }
        return .success(())
    }

    enum StoreError: Error, Equatable {
        case invalid(ValidationError)
        case keychain(KeychainError)
        var text: String {
            switch self {
            case .invalid(let e): return "not stored: \(e.text)"
            case .keychain(let e): return "Keychain: \(e.text)"
            }
        }
    }

    // MARK: child environments

    /// Environment for `flashtex-bridge`: the app's environment with every
    /// credential-, token- or proxy-like variable removed, plus the provider's
    /// key variable (and the bridge's model variable) only when a provider is
    /// selected and a key resolved. Without one the bridge runs without any
    /// provider, minus stray secrets.
    static func bridgeEnvironment(provider: ConversionProvider, credential: Resolution?, model: String,
                                  from environment: [String: String] = ProcessInfo.processInfo.environment) -> [String: String] {
        var env = environment.filter { !isSensitiveVariable($0.key) }
        env.removeValue(forKey: bridgeModelVariable)
        for name in [modelVariable] + legacyModelVariables { env.removeValue(forKey: name) }
        if let credential, let variable = provider.bridgeKeyVariable {
            credential.inject(into: &env, as: variable)
            env[bridgeModelVariable] = model
        }
        return env
    }

    /// Names that never reach a child process unless this adapter puts them
    /// there deliberately.
    static func isSensitiveVariable(_ name: String) -> Bool {
        let upper = name.uppercased()
        return upper == environmentName || upper.contains("API_KEY") || upper.contains("APIKEY")
            || upper.contains("TOKEN") || upper.contains("SECRET") || upper.contains("PASSWORD")
            || upper.contains("CREDENTIAL") || upper.hasSuffix("_PROXY") || upper == "PROXY"
    }
}

// MARK: - Keychain

enum KeychainError: Error, Equatable {
    case status(OSStatus)
    var text: String {
        if case .status(let s) = self {
            let message = SecCopyErrorMessageString(s, nil).map { $0 as String } ?? "OSStatus \(s)"
            return "\(message) (\(s))"
        }
        return "unknown"
    }
}

/// Generic-password storage; `SecItemKeychain` is the real one, tests may use
/// `MemoryKeychain`. `read` returns `.success(nil)` when no item exists.
protocol ConversionKeychainStore {
    func read(service: String, account: String) -> Result<String?, KeychainError>
    func write(_ secret: String, service: String, account: String) -> Result<Void, KeychainError>
    func delete(service: String, account: String) -> Result<Void, KeychainError>
}

/// `SecItem*` against the login keychain (the legacy file-based keychain; the
/// data-protection keychain needs an entitled, signed app). No third-party code.
/// After an ad-hoc re-sign of a rebuilt app, macOS may ask once whether the new
/// build may read the item; that prompt is the system's, not this app's.
struct SecItemKeychain: ConversionKeychainStore {
    static let shared = SecItemKeychain()

    private func query(service: String, account: String) -> [String: Any] {
        [kSecClass as String: kSecClassGenericPassword,
         kSecAttrService as String: service,
         kSecAttrAccount as String: account]
    }

    func read(service: String, account: String) -> Result<String?, KeychainError> {
        var q = query(service: service, account: account)
        q[kSecReturnData as String] = true
        q[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(q as CFDictionary, &item)
        switch status {
        case errSecSuccess:
            guard let data = item as? Data else { return .success(nil) }
            return .success(String(decoding: data, as: UTF8.self))
        case errSecItemNotFound: return .success(nil)
        default: return .failure(.status(status))
        }
    }

    func write(_ secret: String, service: String, account: String) -> Result<Void, KeychainError> {
        let data = Data(secret.utf8)
        let q = query(service: service, account: account)
        let update = SecItemUpdate(q as CFDictionary, [kSecValueData as String: data] as CFDictionary)
        if update == errSecSuccess { return .success(()) }
        guard update == errSecItemNotFound else { return .failure(.status(update)) }
        var add = q
        add[kSecValueData as String] = data
        add[kSecAttrLabel as String] = "FlashTeX capture-conversion API key"
        let status = SecItemAdd(add as CFDictionary, nil)
        return status == errSecSuccess ? .success(()) : .failure(.status(status))
    }

    func delete(service: String, account: String) -> Result<Void, KeychainError> {
        let status = SecItemDelete(query(service: service, account: account) as CFDictionary)
        return status == errSecSuccess || status == errSecItemNotFound ? .success(()) : .failure(.status(status))
    }
}

/// In-memory store for tests (no Keychain access, no prompts).
final class MemoryKeychain: ConversionKeychainStore {
    private var items: [String: String] = [:]
    private let lock = NSLock()
    var failWith: KeychainError?
    init(_ initial: [String: String] = [:]) { items = initial }
    private func k(_ s: String, _ a: String) -> String { s + "\u{0}" + a }
    func read(service: String, account: String) -> Result<String?, KeychainError> {
        if let failWith { return .failure(failWith) }
        return .success(lock.withLock { items[k(service, account)] })
    }
    func write(_ secret: String, service: String, account: String) -> Result<Void, KeychainError> {
        if let failWith { return .failure(failWith) }
        lock.withLock { items[k(service, account)] = secret }
        return .success(())
    }
    func delete(service: String, account: String) -> Result<Void, KeychainError> {
        if let failWith { return .failure(failWith) }
        _ = lock.withLock { items.removeValue(forKey: k(service, account)) }
        return .success(())
    }
    var isEmpty: Bool { lock.withLock { items.isEmpty } }
    /// Introspection for tests: whether an item exists (never its value).
    func has(service: String, account: String) -> Bool { lock.withLock { items[k(service, account)] != nil } }
}

// MARK: - Preferences (provider selection, model)

/// The non-secret capture-conversion settings, in `UserDefaults` under their
/// own keys (separate from `EditorPreferences` so the editor schema is
/// untouched). `FLASHTEX_CONVERSION_PROVIDER` / `FLASHTEX_CONVERSION_MODEL` in
/// the environment take precedence over these at bridge-launch time.
final class ConversionPreferences {
    static let shared = ConversionPreferences(defaults: .standard)
    static let providerKey = "FlashTeX.Conversion.v1.provider"
    static func modelKey(for provider: ConversionProvider) -> String { "FlashTeX.Conversion.v1.model.\(provider.rawValue)" }
    /// Pre-provider keys (older builds): the xAI model id and the assistant
    /// provider mode. The model is migrated; the mode is dropped (the
    /// assistant no longer exists).
    static let legacyModelKey = "FlashTeX.Grok.v1.model"
    static let legacyProviderKeys = ["FlashTeX.Grok.v1.providerEnabled", "FlashTeX.Grok.v1.providerMode"]
    /// Posted (main queue) after any setter so listeners re-read.
    static let didChange = Notification.Name("FlashTeX.Conversion.preferencesDidChange")
    private let defaults: UserDefaults

    init(defaults: UserDefaults) { self.defaults = defaults }

    /// The selected provider; xAI by default so an existing key keeps
    /// converting captures (without a key the bridge simply refuses
    /// `capture_convert`, exactly as before).
    var provider: ConversionProvider {
        get {
            if let raw = defaults.string(forKey: Self.providerKey), let p = ConversionProvider(rawValue: raw) { return p }
            return .xai
        }
        set {
            defaults.set(newValue.rawValue, forKey: Self.providerKey)
            for key in Self.legacyProviderKeys { defaults.removeObject(forKey: key) }
            notify()
        }
    }

    private func notify() {
        NotificationCenter.default.post(name: Self.didChange, object: self)
    }

    /// nil means "use the provider's default model".
    func model(for provider: ConversionProvider) -> String? {
        if let stored = defaults.string(forKey: Self.modelKey(for: provider)) {
            return ConversionCredential.isValidModel(stored) ? stored : nil
        }
        if provider == .xai, let legacy = defaults.string(forKey: Self.legacyModelKey), ConversionCredential.isValidModel(legacy) {
            return legacy
        }
        return nil
    }

    func setModel(_ model: String?, for provider: ConversionProvider) {
        if let model, ConversionCredential.isValidModel(model), model != provider.defaultModel {
            defaults.set(model, forKey: Self.modelKey(for: provider))
        } else {
            defaults.removeObject(forKey: Self.modelKey(for: provider))
        }
        if provider == .xai { defaults.removeObject(forKey: Self.legacyModelKey) }
        notify()
    }
}
