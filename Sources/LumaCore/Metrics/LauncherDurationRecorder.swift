import Foundation

/// Rolling duration samples for launcher/module hot paths (no query or secret content).
public enum LauncherDurationRecorder {
    public enum Category: String, Sendable {
        case moduleWarmup = "module.warmup"
        case moduleHandle = "module.handle"
        case actionPerform = "action.perform"
        case panelHide = "panel.hide"
    }

    private static let lock = NSLock()
    private nonisolated(unsafe) static var samples: [String: [Double]] = [:]
    private static let maxSamplesPerKey = 100

    public static func durationMilliseconds(_ duration: Duration) -> Double {
        let components = duration.components
        return Double(components.seconds) * 1000
            + Double(components.attoseconds) / 1_000_000_000_000_000
    }

    public static func record(category: Category, key: String, milliseconds: Double) {
        let composite = "\(category.rawValue).\(key)"
        lock.lock()
        var bucket = samples[composite, default: []]
        bucket.append(milliseconds)
        if bucket.count > maxSamplesPerKey {
            bucket.removeFirst(bucket.count - maxSamplesPerKey)
        }
        samples[composite] = bucket
        lock.unlock()
    }

    public static func p95(for compositeKey: String) -> Double? {
        lock.lock()
        defer { lock.unlock() }
        guard let bucket = samples[compositeKey], !bucket.isEmpty else { return nil }
        return percentile(bucket, p: 0.95)
    }

    public static func exportSummary() -> [String: Double] {
        lock.lock()
        defer { lock.unlock() }
        var summary: [String: Double] = [:]
        for (key, bucket) in samples where !bucket.isEmpty {
            summary["\(key).p95"] = percentile(bucket, p: 0.95)
            summary["\(key).count"] = Double(bucket.count)
        }
        return summary
    }

    public static func exportSamples() -> [String: [Double]] {
        lock.lock()
        defer { lock.unlock() }
        return samples
    }

    public static func reset() {
        lock.lock()
        samples.removeAll(keepingCapacity: true)
        lock.unlock()
    }

    private static func percentile(_ values: [Double], p: Double) -> Double {
        let sorted = values.sorted()
        let index = Int((Double(sorted.count - 1) * p).rounded())
        return sorted[max(0, min(sorted.count - 1, index))]
    }
}
