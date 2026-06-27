#!/usr/bin/env bash
# Request Reminders access for Luma (shows system dialog if notDetermined).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TMP="$ROOT/.build/reminders-request.swift"
mkdir -p "$(dirname "$TMP")"
cat > "$TMP" <<'SWIFT'
import EventKit
import Foundation

let store = EKEventStore()
let sem = DispatchSemaphore(value: 0)
var status = EKEventStore.authorizationStatus(for: .reminder)

if #available(macOS 14.0, *) {
    Task {
        do {
            let granted = try await store.requestFullAccessToReminders()
            status = granted ? .fullAccess : .denied
        } catch {
            fputs("request failed: \(error)\n", stderr)
        }
        sem.signal()
    }
    sem.wait()
} else {
    store.requestAccess(to: .reminder) { granted, _ in
        status = granted ? .authorized : .denied
        sem.signal()
    }
    sem.wait()
}

print("reminders_status=\(status.rawValue)")

SWIFT

swift "$TMP"
