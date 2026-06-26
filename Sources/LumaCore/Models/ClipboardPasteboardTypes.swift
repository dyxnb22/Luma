import Foundation

/// Pure UTI / pasteboard type classification shared by modules and services.
public enum ClipboardPasteboardTypes {
    public static func isImageTypes(_ types: [String]) -> Bool {
        let imagePrefixes = ["public.png", "public.tiff", "public.jpeg", "public.image", "com.apple.pict", "com.compuserve.gif"]
        return types.contains { type in
            imagePrefixes.contains { type.hasPrefix($0) || type == $0 }
        }
    }

    public static func isTextTypes(_ types: [String]) -> Bool {
        let textTypes: Set<String> = [
            "public.utf8-plain-text",
            "NSStringPboardType",
            "public.plain-text",
            "public.text",
            "public.html",
            "public.rtf"
        ]
        return types.contains { type in
            textTypes.contains(type) || type.hasPrefix("public.text")
        }
    }

    public static func isFileTypes(_ types: [String]) -> Bool {
        types.contains { $0 == "public.file-url" || $0 == "NSURLPboardType" }
    }
}
