import CoreServices
import Foundation
import LumaCore

public struct FSChangeEvent: Sendable, Hashable {
    public enum Kind: Sendable { case created, removed, renamed, modified, unknown }
    public let path: String
    public let kind: Kind

    public init(path: String, kind: Kind) {
        self.path = path
        self.kind = kind
    }
}

private final class FSEventsCallbackContext: @unchecked Sendable {
    let service: FSEventsService
    let rootPath: String

    init(service: FSEventsService, rootPath: String) {
        self.service = service
        self.rootPath = rootPath
    }
}

public actor FSEventsService: FileSystemClient {
    private struct WatchState {
        var stream: FSEventStreamRef?
        var context: Unmanaged<FSEventsCallbackContext>?
        var continuation: AsyncStream<[FSChangeEvent]>.Continuation?
        var debounceTask: Task<Void, Never>?
        var pendingEvents: [FSChangeEvent] = []
        var debounceMillis: Int
    }

    private var watches: [String: WatchState] = [:]

    public init() {}

    public func watch(root: URL, debounceMillis: Int = 200) -> AsyncStream<[FSChangeEvent]> {
        let rootPath = root.standardizedFileURL.path
        stop(root: root)

        return AsyncStream { continuation in
            Task {
                self.beginWatch(root: root, rootPath: rootPath, debounceMillis: debounceMillis, continuation: continuation)
            }
        }
    }

    public func stop(root: URL) {
        let rootPath = root.standardizedFileURL.path
        guard let state = watches.removeValue(forKey: rootPath) else { return }

        state.debounceTask?.cancel()
        state.continuation?.finish()

        if let stream = state.stream {
            FSEventStreamStop(stream)
            FSEventStreamInvalidate(stream)
            FSEventStreamRelease(stream)
        }
        state.context?.release()
    }

    private func beginWatch(
        root: URL,
        rootPath: String,
        debounceMillis: Int,
        continuation: AsyncStream<[FSChangeEvent]>.Continuation
    ) {
        let context = Unmanaged.passRetained(FSEventsCallbackContext(service: self, rootPath: rootPath))
        let contextPtr = context.toOpaque()

        var streamContext = FSEventStreamContext(
            version: 0,
            info: contextPtr,
            retain: nil,
            release: nil,
            copyDescription: nil
        )

        let flags = FSEventStreamCreateFlags(
            kFSEventStreamCreateFlagFileEvents | kFSEventStreamCreateFlagNoDefer
        )

        guard let stream = FSEventStreamCreate(
            nil,
            fsEventCallback,
            &streamContext,
            [rootPath] as CFArray,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            0.0,
            flags
        ) else {
            context.release()
            continuation.finish()
            return
        }

        watches[rootPath] = WatchState(
            stream: stream,
            context: context,
            continuation: continuation,
            debounceTask: nil,
            pendingEvents: [],
            debounceMillis: debounceMillis
        )

        FSEventStreamSetDispatchQueue(stream, DispatchQueue.global(qos: .utility))
        FSEventStreamStart(stream)

        continuation.onTermination = { @Sendable _ in
            Task { await self.stop(root: root) }
        }
    }

    fileprivate func receiveEvents(_ events: [FSChangeEvent], forRoot rootPath: String) {
        guard var state = watches[rootPath] else { return }

        state.pendingEvents.append(contentsOf: events)

        if state.debounceTask == nil {
            let millis = state.debounceMillis
            state.debounceTask = Task { [rootPath] in
                try? await Task.sleep(for: .milliseconds(millis))
                self.flushPending(forRoot: rootPath)
            }
        }

        watches[rootPath] = state
    }

    private func flushPending(forRoot rootPath: String) {
        guard var state = watches[rootPath] else { return }

        let batch = state.pendingEvents
        state.pendingEvents = []
        state.debounceTask = nil
        watches[rootPath] = state

        guard !batch.isEmpty else { return }
        state.continuation?.yield(batch)
    }
}

private func fsEventCallback(
    _ streamRef: ConstFSEventStreamRef,
    clientCallBackInfo: UnsafeMutableRawPointer?,
    numEvents: Int,
    eventPaths: UnsafeMutableRawPointer,
    eventFlags: UnsafePointer<FSEventStreamEventFlags>,
    eventIds: UnsafePointer<FSEventStreamEventId>
) {
    guard let clientCallBackInfo else { return }
    let context = Unmanaged<FSEventsCallbackContext>.fromOpaque(clientCallBackInfo).takeUnretainedValue()

    let pathsPtr = eventPaths.assumingMemoryBound(to: UnsafePointer<CChar>.self)
    var events: [FSChangeEvent] = []

    for index in 0..<numEvents {
        let path = String(cString: pathsPtr[index])
        guard shouldEmitEvent(for: path) else { continue }
        let flags = eventFlags[index]
        let kind = mapKind(flags: flags)
        events.append(FSChangeEvent(path: path, kind: kind))
    }

    guard !events.isEmpty else { return }
    let rootPath = context.rootPath
    let service = context.service
    Task { await service.receiveEvents(events, forRoot: rootPath) }
}

private func shouldEmitEvent(for path: String) -> Bool {
    let url = URL(fileURLWithPath: path)
    let ext = url.pathExtension
    if ext.isEmpty {
        var isDirectory: ObjCBool = false
        if FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory), isDirectory.boolValue {
            return true
        }
        return false
    }
    return ext.compare("md", options: .caseInsensitive) == .orderedSame
}

private func mapKind(flags: FSEventStreamEventFlags) -> FSChangeEvent.Kind {
    if flags & UInt32(kFSEventStreamEventFlagItemCreated) != 0 { return .created }
    if flags & UInt32(kFSEventStreamEventFlagItemRemoved) != 0 { return .removed }
    if flags & UInt32(kFSEventStreamEventFlagItemRenamed) != 0 { return .renamed }
    if flags & UInt32(kFSEventStreamEventFlagItemModified) != 0 { return .modified }
    return .unknown
}
