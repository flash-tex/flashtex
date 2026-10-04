import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// The capture-conversion provider seam on the Mac side
/// (ConversionCredential.swift, ShellModel+Bridge.swift `bridgeConversionLaunch`).
/// Nothing here contacts a provider: the credential adapter is exercised
/// against an in-memory store (plus one real Keychain round trip under a
/// throwaway service name) and the bridge launch against `Fixtures/fake_bridge.py`.
@MainActor
final class ConversionCredentialTests: XCTestCase {
    static let envOnly = ["FLASHTEX_KEYCHAIN_OFF": "1"]

    private func temporaryDefaults() -> UserDefaults {
        let name = "flashtex.conversion.tests.\(UUID().uuidString)"
        let d = UserDefaults(suiteName: name)!
        d.removePersistentDomain(forName: name)
        return d
    }

    // MARK: credential adapter

    func testResolutionOrderIsKeychainThenEnvironmentThenNone() throws {
        let keychain = MemoryKeychain([ConversionProvider.xai.keychainService + "\u{0}" + ConversionCredential.keychainAccount: " kc-key \n"])
        let env = ["FLASHTEX_AI_API_KEY": "generic-key", "XAI_API_KEY": "env-key", "PATH": "/usr/bin"]
        let fromKeychain = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: env, keychain: keychain))
        XCTAssertEqual(fromKeychain.source, .keychain)
        XCTAssertNil(fromKeychain.variable)
        XCTAssertEqual(fromKeychain.keyByteCount, 6, "trimmed")
        var out: [String: String] = [:]
        fromKeychain.inject(into: &out, as: "K")
        XCTAssertEqual(out, ["K": "kc-key"])

        // The provider-neutral name comes first, then the provider's own names.
        let fromEnv = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: env, keychain: MemoryKeychain()))
        XCTAssertEqual(fromEnv.source, .environment)
        XCTAssertEqual(fromEnv.variable, "FLASHTEX_AI_API_KEY")
        let fromLegacyVar = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: ["XAI_API_KEY": "env-key"], keychain: MemoryKeychain()))
        XCTAssertEqual(fromLegacyVar.variable, "XAI_API_KEY")
        let fromHelperVar = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: ["FLASHTEX_GROK_API_KEY": "helper-key"], keychain: MemoryKeychain()))
        XCTAssertEqual(fromHelperVar.variable, "FLASHTEX_GROK_API_KEY")

        // FLASHTEX_KEYCHAIN_OFF=1 skips the store even when it holds a key.
        let skipped = ConversionCredential.status(for: .xai, environment: Self.envOnly, keychain: keychain)
        XCTAssertNil(skipped.resolution)
        XCTAssertTrue(skipped.keychainSkipped)
        XCTAssertTrue(skipped.text.hasPrefix("xAI API key: absent"), skipped.text)
        let envWhileSkipped = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: ["FLASHTEX_KEYCHAIN_OFF": "1", "XAI_API_KEY": "e"], keychain: keychain))
        XCTAssertEqual(envWhileSkipped.source, .environment)

        // Provider "none" never resolves anything, even with a key around.
        XCTAssertNil(ConversionCredential.resolve(for: .none, environment: env, keychain: keychain))
        XCTAssertEqual(ConversionCredential.status(for: .none, environment: env, keychain: keychain).text, "conversion disabled: no provider selected")

        XCTAssertNil(ConversionCredential.resolve(for: .xai, environment: ["PATH": "/usr/bin"], keychain: MemoryKeychain()))
        // Unusable values are not credentials.
        XCTAssertNil(ConversionCredential.resolve(for: .xai, environment: ["XAI_API_KEY": "  \n"], keychain: MemoryKeychain()))
        XCTAssertNil(ConversionCredential.resolve(for: .xai, environment: ["XAI_API_KEY": "bad\u{01}key"], keychain: MemoryKeychain()))
        XCTAssertEqual(ConversionCredential.validate(String(repeating: "x", count: 8193)), .failure(.tooLong))
        // A Keychain failure is reported, not thrown, and the environment still wins.
        let broken = MemoryKeychain(); broken.failWith = .status(errSecInteractionNotAllowed)
        let status = ConversionCredential.status(for: .xai, environment: ["XAI_API_KEY": "e"], keychain: broken)
        XCTAssertEqual(status.resolution?.source, .environment)
        XCTAssertNotNil(status.keychainProblem)
        let none = ConversionCredential.status(for: .xai, environment: [:], keychain: broken)
        XCTAssertTrue(none.text.contains("(") && none.text.contains("-25308"), none.text)
    }

    /// The pre-provider item (`tech.jay3332.flashtex.xai`, account `xai` or
    /// `XAI_API_KEY`) is moved to `tech.jay3332.flashtex.ai.xai` on first read.
    func testLegacyKeychainItemIsMigratedOnFirstRead() throws {
        for account in ConversionCredential.legacyXAIKeychainAccounts {
            let keychain = MemoryKeychain([ConversionCredential.legacyXAIKeychainService + "\u{0}" + account: "legacy-key"])
            let r = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: [:], keychain: keychain))
            XCTAssertEqual(r.source, .keychain)
            XCTAssertEqual(r.keyByteCount, 10)
            XCTAssertTrue(keychain.has(service: ConversionProvider.xai.keychainService, account: ConversionCredential.keychainAccount), account)
            XCTAssertFalse(keychain.has(service: ConversionCredential.legacyXAIKeychainService, account: account), "old item removed (\(account))")
            // Second read: the migrated item is used directly.
            XCTAssertEqual(ConversionCredential.resolve(for: .xai, environment: [:], keychain: keychain)?.source, .keychain)
        }
        // The new item wins over a lingering legacy one; remove clears both.
        let both = MemoryKeychain([ConversionProvider.xai.keychainService + "\u{0}" + ConversionCredential.keychainAccount: "new-key-1",
                                   ConversionCredential.legacyXAIKeychainService + "\u{0}xai": "old-key"])
        XCTAssertEqual(ConversionCredential.resolve(for: .xai, environment: [:], keychain: both)?.keyByteCount, 9)
        assertOK(ConversionCredential.remove(for: .xai, keychain: both))
        XCTAssertTrue(both.isEmpty)
    }

    func testNoTextEverContainsTheKey() throws {
        let secret = "xai-SUPER-SECRET-VALUE-0123456789"
        let keychain = MemoryKeychain()
        assertOK(ConversionCredential.store(secret, for: .xai, keychain: keychain))
        let status = ConversionCredential.status(for: .xai, environment: [:], keychain: keychain)
        let r = try XCTUnwrap(status.resolution)
        for text in [r.description, r.debugDescription, "\(r)", String(reflecting: r), status.text] {
            XCTAssertFalse(text.contains(secret), text)
            XCTAssertFalse(text.contains("SUPER"), text)
        }
        XCTAssertEqual(r.description, "key present (Keychain)")
        XCTAssertEqual(status.text, "xAI API key: key present (Keychain)")
        let envR = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: ["XAI_API_KEY": secret], keychain: MemoryKeychain()))
        XCTAssertEqual(envR.description, "key present (environment: XAI_API_KEY)")
        // Removal.
        assertOK(ConversionCredential.remove(for: .xai, keychain: keychain))
        XCTAssertTrue(keychain.isEmpty)
        XCTAssertNil(ConversionCredential.resolve(for: .xai, environment: [:], keychain: keychain))
        if case .failure(.invalid(.empty)) = ConversionCredential.store("", for: .xai, keychain: keychain) {} else { XCTFail("empty key stored") }
        if case .failure(.invalid(.empty)) = ConversionCredential.store("k", for: .none, keychain: keychain) {} else { XCTFail("key stored for provider none") }
        XCTAssertTrue(keychain.isEmpty)
    }

    /// The real `SecItem` store, under a throwaway service so the user's own
    /// item (`tech.jay3332.flashtex.ai.xai`) is never read or written by tests.
    func testSecItemKeychainRoundTripUnderAThrowawayService() throws {
        let service = "tech.jay3332.flashtex.ai.test-\(UUID().uuidString.lowercased())"
        let account = ConversionCredential.keychainAccount
        let store = SecItemKeychain()
        defer { _ = store.delete(service: service, account: account) }
        XCTAssertEqual(try store.read(service: service, account: account).get(), nil)
        switch store.write("first-key", service: service, account: account) {
        case .failure(let e): throw XCTSkip("Keychain unavailable to this test process: \(e.text)")
        case .success: break
        }
        XCTAssertEqual(try store.read(service: service, account: account).get(), "first-key")
        // Update in place (SecItemUpdate path), then delete; delete is idempotent.
        assertOK(store.write("second-key", service: service, account: account))
        XCTAssertEqual(try store.read(service: service, account: account).get(), "second-key")
        assertOK(store.delete(service: service, account: account))
        XCTAssertEqual(try store.read(service: service, account: account).get(), nil)
        assertOK(store.delete(service: service, account: account))
    }

    func testProviderAndModelSelection() {
        let defaults = temporaryDefaults()
        let prefs = ConversionPreferences(defaults: defaults)
        XCTAssertEqual(prefs.provider, .xai, "default: the existing xAI path (off without a key)")
        XCTAssertNil(prefs.model(for: .xai))
        XCTAssertEqual(ConversionProvider.xai.defaultModel, "grok-4.20-0309-non-reasoning")
        XCTAssertEqual(ConversionCredential.model(for: .xai, environment: [:], preferences: prefs), "grok-4.20-0309-non-reasoning")
        XCTAssertEqual(ConversionCredential.provider(environment: [:], preferences: prefs), .xai)
        XCTAssertEqual(ConversionCredential.provider(environment: ["FLASHTEX_CONVERSION_PROVIDER": "none"], preferences: prefs), .none, "environment wins")
        XCTAssertEqual(ConversionCredential.provider(environment: ["FLASHTEX_CONVERSION_PROVIDER": "XAI"], preferences: prefs), .xai)
        XCTAssertEqual(ConversionCredential.provider(environment: ["FLASHTEX_CONVERSION_PROVIDER": "nope"], preferences: prefs), .xai, "unknown value ignored")
        prefs.setModel("grok-4.6", for: .xai)
        XCTAssertEqual(ConversionCredential.model(for: .xai, environment: [:], preferences: prefs), "grok-4.6")
        XCTAssertEqual(ConversionCredential.model(for: .xai, environment: ["FLASHTEX_CONVERSION_MODEL": "grok-x"], preferences: prefs), "grok-x", "environment wins")
        XCTAssertEqual(ConversionCredential.model(for: .xai, environment: ["FLASHTEX_GROK_CAPTURE_MODEL": "grok-y"], preferences: prefs), "grok-y", "legacy name honoured")
        XCTAssertEqual(ConversionCredential.model(for: .xai, environment: ["FLASHTEX_CONVERSION_MODEL": "bad model!"], preferences: prefs), "grok-4.6", "invalid env value ignored")
        prefs.setModel(ConversionProvider.xai.defaultModel, for: .xai)
        XCTAssertNil(prefs.model(for: .xai), "the default is not persisted")
        prefs.setModel("not valid", for: .xai)
        XCTAssertNil(prefs.model(for: .xai))
        // Legacy model preference migrates; legacy assistant mode keys are dropped on write.
        let legacy = temporaryDefaults()
        legacy.set("grok-4.6", forKey: ConversionPreferences.legacyModelKey)
        legacy.set("on", forKey: "FlashTeX.Grok.v1.providerMode")
        let migrated = ConversionPreferences(defaults: legacy)
        XCTAssertEqual(migrated.model(for: .xai), "grok-4.6")
        migrated.provider = .none
        XCTAssertNil(legacy.object(forKey: "FlashTeX.Grok.v1.providerMode"))
        XCTAssertEqual(ConversionPreferences(defaults: legacy).provider, .none)
        var changes = 0
        let token = NotificationCenter.default.addObserver(forName: ConversionPreferences.didChange, object: nil, queue: nil) { _ in changes += 1 }
        prefs.provider = .xai
        prefs.setModel("grok-4.6", for: .xai)
        NotificationCenter.default.removeObserver(token)
        XCTAssertEqual(changes, 2, "listeners re-read on every preference change")
        XCTAssertTrue(ConversionCredential.isValidModel("grok-4.6"))
        XCTAssertFalse(ConversionCredential.isValidModel(""))
        XCTAssertFalse(ConversionCredential.isValidModel(String(repeating: "a", count: 129)))
    }

    func testBridgeEnvironmentStripsSecretsAndAddsOnlyTheProviderKey() throws {
        let env = ["PATH": "/usr/bin", "HOME": "/tmp", "OPENAI_API_KEY": "leak", "GITHUB_TOKEN": "leak", "HTTPS_PROXY": "leak",
                   "FLASHTEX_AI_API_KEY": "k", "FLASHTEX_GROK_MODEL": "stale", "FLASHTEX_CONVERSION_MODEL": "grok-4.6"]
        let credential = try XCTUnwrap(ConversionCredential.resolve(for: .xai, environment: env + Self.envOnly, keychain: MemoryKeychain()))
        let withKey = ConversionCredential.bridgeEnvironment(provider: .xai, credential: credential, model: "grok-4.6", from: env)
        XCTAssertEqual(withKey, ["PATH": "/usr/bin", "HOME": "/tmp", "XAI_API_KEY": "k", "FLASHTEX_GROK_MODEL": "grok-4.6"])
        let without = ConversionCredential.bridgeEnvironment(provider: .none, credential: nil, model: "", from: env)
        XCTAssertEqual(without, ["PATH": "/usr/bin", "HOME": "/tmp"])
        // A credential with provider none never reaches the child.
        XCTAssertEqual(ConversionCredential.bridgeEnvironment(provider: .none, credential: credential, model: "m", from: env), without)
    }

    // MARK: the bridge, launched through the shell with the key in its environment

    func testBridgeConversionUsesTheProviderFlagAndTheKeyOnlyWhenPresent() async throws {
        let model = ShellModel()
        model.autoCompile = false
        let store = try BridgeClientTests.tempStore()
        let path = ProcessInfo.processInfo.environment["PATH"] ?? "/usr/bin"
        // Without a key: no provider flag and provider_disabled becomes plain guidance.
        let noKey = ShellModel.bridgeConversionLaunch(environment: ["FLASHTEX_KEYCHAIN_OFF": "1", "PATH": path],
                                                      keychain: MemoryKeychain(), preferences: ConversionPreferences(defaults: temporaryDefaults()))
        XCTAssertEqual(noKey.provider, .none)
        XCTAssertNil(noKey.credential)
        var ok = await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path], storeDirectory: store,
                                                 provider: noKey.provider, environment: noKey.environment, ledger: ShellModelBridgeTests.fakeLedger, discoverLedger: false)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
        XCTAssertEqual(model.bridge?.conversionEnabled, false)
        model.caretUTF16 = 0
        model.pinAnchorAtCaret()
        try await waitUntil("pin \(model.bridgeStatus) / \(model.captureNote ?? "")") { model.bridgeDestination != nil }
        let image = try BridgeClientTests.fixtureCapture().image
        let received = await model.submitCapture(image: image, captureId: "conv-gate-1", instructions: "%grokgate"); XCTAssertNotNil(received, model.captureNote ?? "")
        let refused = await model.convertCapture(captureId: "conv-gate-1"); XCTAssertNil(refused)
        XCTAssertTrue(model.captureNote?.contains("provider_disabled") == true, model.captureNote ?? "")
        XCTAssertTrue(model.captureNote?.contains("Preferences (⌘,)") == true, model.captureNote ?? "")
        model.detachBridge()

        // Provider "none" with a key present: still no flag, no key on the child.
        let disabled = ShellModel.bridgeConversionLaunch(environment: ["FLASHTEX_KEYCHAIN_OFF": "1", "FLASHTEX_AI_API_KEY": "k", "FLASHTEX_CONVERSION_PROVIDER": "none", "PATH": path],
                                                         keychain: MemoryKeychain(), preferences: ConversionPreferences(defaults: temporaryDefaults()))
        XCTAssertEqual(disabled.provider, .none)
        XCTAssertNil(disabled.environment["XAI_API_KEY"])
        XCTAssertNil(disabled.environment["FLASHTEX_AI_API_KEY"])

        // With a key: the provider flag, its key variable and the model reach the bridge; the proposal is reviewed as usual.
        let withKey = ShellModel.bridgeConversionLaunch(environment: ["FLASHTEX_KEYCHAIN_OFF": "1", "FLASHTEX_AI_API_KEY": "fixture-bridge-key", "FLASHTEX_CONVERSION_MODEL": "grok-4.6-vision",
                                                                      "PATH": path],
                                                        keychain: MemoryKeychain(), preferences: ConversionPreferences(defaults: temporaryDefaults()))
        XCTAssertEqual(withKey.provider, .xai)
        XCTAssertEqual(withKey.environment["XAI_API_KEY"], "fixture-bridge-key")
        XCTAssertNil(withKey.environment["FLASHTEX_AI_API_KEY"], "only the bridge's own variable")
        ok = await model.attachBridgeAndWait(executable: BridgeClientTests.python, arguments: [BridgeClientTests.fakeBridge.path], storeDirectory: store,
                                             provider: withKey.provider, environment: withKey.environment, ledger: ShellModelBridgeTests.fakeLedger, discoverLedger: false)
        XCTAssertTrue(ok, model.captureNote ?? model.bridgeStatus)
        XCTAssertEqual(model.bridge?.conversionEnabled, true)
        XCTAssertEqual(model.bridge?.conversionProvider, .xai)
        model.pinAnchorAtCaret() // a new bridge process holds no destination; pin again as the user would
        try await waitUntil("pin \(model.bridgeStatus) / \(model.captureNote ?? "")") { model.bridgeDestination != nil }
        let received2 = await model.submitCapture(image: image, captureId: "conv-gate-2", instructions: "%grokgate"); XCTAssertNotNil(received2, model.captureNote ?? "")
        let converted = await model.convertCapture(captureId: "conv-gate-2"); let proposal = try XCTUnwrap(converted, model.captureNote ?? "")
        XCTAssertEqual(proposal.latex, "\\fakecapture{conv-gate-2}")
        XCTAssertTrue(proposal.ambiguities.contains("model:grok-4.6-vision key:present"), "\(proposal.ambiguities)")
        XCTAssertEqual(model.proposals.count, 1, "queued for the review sheet; nothing inserted")
        XCTAssertEqual(model.activeText, "Hello FlashTeX.\n")
        // Bridge provider errors map to guidance without the key.
        let note = ShellModel.conversionFailureNote(BridgeClient.Failure.bridge(.init(code: "provider_auth_error", message: "provider returned HTTP 401")), provider: .xai)
        XCTAssertTrue(note.contains("401/403") && note.contains("Capture conversion"), note)
        XCTAssertTrue(ShellModel.conversionFailureNote(BridgeClient.Failure.bridge(.init(code: "provider_rate_limited", message: "")), provider: .xai).contains("429"))
        XCTAssertTrue(ShellModel.conversionFailureNote(BridgeClient.Failure.bridge(.init(code: "provider_timeout", message: "")), provider: .xai).contains("90 s"))
        XCTAssertTrue(ShellModel.conversionFailureNote(BridgeClient.Failure.timeout(3), provider: .xai).hasPrefix("Conversion unavailable (xAI enabled): no reply"))
        model.detachBridge()
    }

    /// Waits on a helper process (the Python bridge double); generous because a loaded runner
    /// delays process start-up and replies by seconds, not because anything is timed.
    private func waitUntil(_ what: String, timeout: TimeInterval = 30, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 20_000_000)
        }
        XCTFail("timed out after \(Int(timeout)) s waiting for \(what) from the bridge double")
    }
}

private func assertOK<E: Error>(_ result: Result<Void, E>, file: StaticString = #filePath, line: UInt = #line) {
    if case .failure(let e) = result { XCTFail("\(e)", file: file, line: line) }
}

private func + (lhs: [String: String], rhs: [String: String]) -> [String: String] {
    lhs.merging(rhs) { _, new in new }
}
