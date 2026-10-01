import Foundation
#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// A `COMPILE` request (spec §6.3).
public struct DL3CompileRequest: Sendable {
    public struct Edit: Sendable, Equatable {
        public var path: String, offset: Int, delete: Int, insert: String
        public init(path: String, offset: Int, delete: Int, insert: String) {
            self.path = path; self.offset = offset; self.delete = delete; self.insert = insert
        }
    }
    public var id: Int
    public var root: String
    public var main: String
    public var format = "pdflatex"
    public var shellEscape = "default"
    public var outputDir: String?
    public var jobname: String?
    public var haveFonts: [String] = []
    /// Font formats beyond type1/none whose programs the client draws
    /// (`font_formats`, host capability `font-formats`, lane P3-FONTS-2).
    public var fontFormats: [String] = []
    public var incremental = true
    public var viewport: Int?
    public var buffers: [(path: String, text: String)] = []
    public var edits: [Edit] = []
    public var export = false
    /// 3.2: `auto` runs bibtex, biber and makeindex as latexmk would (a
    /// trusted project only, DESIGN.md §4.5); `off` never; nil: the host's default.
    public var externalTools: String?

    public init(id: Int, root: String, main: String) { self.id = id; self.root = root; self.main = main }

    public var json: DL3JSON {
        var o: [String: DL3JSON] = ["id": .int(Int64(id)), "root": .string(root), "main": .string(main),
                                    "format": .string(format), "shell_escape": .string(shellEscape)]
        if let outputDir { o["output_dir"] = .string(outputDir) }
        if let jobname { o["jobname"] = .string(jobname) }
        if !haveFonts.isEmpty { o["have_fonts"] = .array(haveFonts.map(DL3JSON.string)) }
        if !fontFormats.isEmpty { o["font_formats"] = .array(fontFormats.map(DL3JSON.string)) }
        if incremental { o["incremental"] = .bool(true) }
        if let viewport { o["viewport"] = .int(Int64(viewport)) }
        if !buffers.isEmpty { o["buffers"] = .array(buffers.map { .object(["path": .string($0.path), "text": .string($0.text)]) }) }
        if !edits.isEmpty {
            o["edits"] = .array(edits.map { .object(["path": .string($0.path), "offset": .int(Int64($0.offset)),
                                                     "delete": .int(Int64($0.delete)), "insert": .string($0.insert)]) })
        }
        if export { o["export"] = .bool(true) }
        if let externalTools { o["external_tools"] = .string(externalTools) }
        return .object(o)
    }
}

/// One connection to the engine host's Unix socket (spec §6): HELLO, then
/// requests out and decoded events in. Events are decoded on the
/// connection's own reader thread and handed to `onEvent` there, in stream
/// order; a decoding or I/O failure ends the connection (`onClose` with the
/// error), per spec §7 (fail closed, reconnect and recompile).
public final class DL3Connection: @unchecked Sendable {
    public let hello: DL3JSON
    private let fd: Int32
    private let writeLock = NSLock()
    private var closed = false
    private let stateLock = NSLock()

    /// Connects and exchanges HELLO (blocking; call off the main thread).
    /// `accept`: optional message families to receive (`HELLO.accept`, e.g.
    /// `DL3Diag.capability`); a host that does not offer one ignores it.
    public init(socketPath: String, client: String = "FlashTeX", accept: [String] = []) throws {
        let fd = socket(AF_UNIX, Int32(SOCK_STREAM), 0)
        guard fd >= 0 else { throw DL3Error("socket: \(String(cString: strerror(errno)))") }
        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        let pathBytes = Array(socketPath.utf8)
        let capacity = MemoryLayout.size(ofValue: addr.sun_path)
        guard pathBytes.count < capacity else { close(fd); throw DL3Error("socket path too long: \(socketPath)") }
        withUnsafeMutableBytes(of: &addr.sun_path) { raw in
            raw.copyBytes(from: pathBytes)
            raw[pathBytes.count] = 0
        }
        let ok = withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) { connect(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size)) }
        }
        guard ok == 0 else {
            let e = String(cString: strerror(errno)); close(fd)
            throw DL3Error("connect \(socketPath): \(e)")
        }
        var big: Int32 = 4 << 20
        setsockopt(fd, SOL_SOCKET, SO_RCVBUF, &big, socklen_t(MemoryLayout<Int32>.size))
        setsockopt(fd, SOL_SOCKET, SO_SNDBUF, &big, socklen_t(MemoryLayout<Int32>.size))
        #if canImport(Darwin)
        var one: Int32 = 1
        setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &one, socklen_t(MemoryLayout<Int32>.size))
        #endif
        self.fd = fd
        var helloFields: [String: DL3JSON] = ["protocol": .string(DL3.protocolName),
                                              "version": .array([.int(Int64(DL3.versionMajor)), .int(Int64(DL3.versionMinor))]),
                                              "client": .string(client)]
        if !accept.isEmpty { helloFields["accept"] = .array(accept.map(DL3JSON.string)) }
        let helloJSON: DL3JSON = .object(helloFields)
        do {
            try Self.writeAll(fd, DL3Frames.encode(kind: DL3.Kind.cHello, body: helloJSON.data()))
            guard let (k, body) = try Self.readFrame(fd) else { throw DL3Error("host closed before HELLO") }
            let j = try DL3JSON.parse(body)
            if k == DL3.Kind.error { throw DL3Error("host refused: \(j["message"]?.string ?? "\(j)")") }
            guard k == DL3.Kind.hello, j["protocol"]?.string == DL3.protocolName else { throw DL3Error("not a display-list-v3 host") }
            guard j["version"]?.array?.first?.int == Int64(DL3.versionMajor) else {
                throw DL3Error("host speaks version \(j["version"].map { "\($0)" } ?? "?"), not \(DL3.versionMajor)")
            }
            hello = j
        } catch {
            close(fd)
            throw error
        }
    }

    deinit { shutdownSocket(); close(fd) }

    /// Starts the reader thread. `onEvent` gets every decoded event in
    /// order; `onClose` once, when the stream ends (nil) or fails.
    public func start(onEvent: @escaping @Sendable (DL3Event) -> Void, onClose: @escaping @Sendable (DL3Error?) -> Void) {
        start(onTimedEvent: { ev, _ in onEvent(ev) }, onClose: onClose)
    }

    /// When a frame was read and decoded on the reader thread
    /// (`DispatchTime` uptime nanoseconds; the frame's first byte may have
    /// waited in the socket buffer before `readNs`).
    public struct Timing: Sendable { public var readNs: UInt64; public var decodedNs: UInt64 }

    /// `start`, with each event's read and decode times.
    public func start(onTimedEvent: @escaping @Sendable (DL3Event, Timing) -> Void, onClose: @escaping @Sendable (DL3Error?) -> Void) {
        let fd = self.fd
        let thread = Thread {
            while true {
                do {
                    guard let (k, body) = try Self.readFrame(fd) else { onClose(nil); return }
                    let read = DispatchTime.now().uptimeNanoseconds
                    let ev = try DL3Event.decode(kind: k, body: body)
                    onTimedEvent(ev, Timing(readNs: read, decodedNs: DispatchTime.now().uptimeNanoseconds))
                } catch let e as DL3Error {
                    onClose(e); return
                } catch {
                    onClose(DL3Error("\(error)")); return
                }
            }
        }
        thread.name = "flashtex.dl3.reader"
        thread.qualityOfService = .userInteractive
        thread.stackSize = 4 << 20
        thread.start()
    }

    public func compile(_ request: DL3CompileRequest) throws { try send(DL3.Kind.compile, request.json) }
    public func cancel(id: Int) throws { try send(DL3.Kind.cancel, .object(["id": .int(Int64(id))])) }
    public func bye() { try? send(DL3.Kind.bye, .object([:])); shutdownSocket() }

    private func send(_ kind: UInt8, _ json: DL3JSON) throws {
        writeLock.lock(); defer { writeLock.unlock() }
        try Self.writeAll(fd, DL3Frames.encode(kind: kind, body: json.data()))
    }

    private func shutdownSocket() {
        stateLock.lock(); defer { stateLock.unlock() }
        if !closed { closed = true; shutdown(fd, Int32(SHUT_RDWR)) }
    }

    static func writeAll(_ fd: Int32, _ data: Data) throws {
        try data.withUnsafeBytes { raw in
            var off = 0
            while off < raw.count {
                let n = write(fd, raw.baseAddress! + off, raw.count - off)
                if n < 0 {
                    if errno == EINTR { continue }
                    throw DL3Error("write: \(String(cString: strerror(errno)))")
                }
                off += n
            }
        }
    }

    /// One frame, or nil at a clean end of stream.
    static func readFrame(_ fd: Int32) throws -> (UInt8, [UInt8])? {
        var head = [UInt8](repeating: 0, count: 5)
        guard try readExactly(fd, &head, allowEOF: true) else { return nil }
        let len = Int(UInt32(head[0]) | UInt32(head[1]) << 8 | UInt32(head[2]) << 16 | UInt32(head[3]) << 24)
        guard len >= 1, len <= DL3.maxFrame else { throw DL3Error("bad frame length \(len)") }
        var body = [UInt8](repeating: 0, count: len - 1)
        if len > 1 { _ = try readExactly(fd, &body, allowEOF: false) }
        return (head[4], body)
    }

    private static func readExactly(_ fd: Int32, _ buf: inout [UInt8], allowEOF: Bool) throws -> Bool {
        var got = 0
        let n = buf.count
        while got < n {
            let r = buf.withUnsafeMutableBytes { read(fd, $0.baseAddress! + got, n - got) }
            if r == 0 {
                if got == 0 && allowEOF { return false }
                throw DL3Error("truncated frame")
            }
            if r < 0 {
                if errno == EINTR { continue }
                throw DL3Error("read: \(String(cString: strerror(errno)))")
            }
            got += r
        }
        return true
    }
}
