import Foundation

public struct ClipboardSnapshot: Sendable, Equatable {
    public let changeCount: Int
    public let types: [String]
    public let text: String?
    public let imageData: Data?
    public let imageType: String?
    public let fileURLs: [URL]
    public let sourceAppName: String?
    public let sourceBundleID: String?
    public let sourceIsLuma: Bool

    public init(
        changeCount: Int,
        types: [String],
        text: String?,
        imageData: Data?,
        imageType: String?,
        fileURLs: [URL],
        sourceAppName: String?,
        sourceBundleID: String?,
        sourceIsLuma: Bool = false
    ) {
        self.changeCount = changeCount
        self.types = types
        self.text = text
        self.imageData = imageData
        self.imageType = imageType
        self.fileURLs = fileURLs
        self.sourceAppName = sourceAppName
        self.sourceBundleID = sourceBundleID
        self.sourceIsLuma = sourceIsLuma
    }
}
