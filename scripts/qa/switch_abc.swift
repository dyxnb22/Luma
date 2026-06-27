#!/usr/bin/env swift
import Carbon

let abcID = "com.apple.keylayout.ABC" as CFString
let list = TISCreateInputSourceList(nil, true).takeRetainedValue() as! [TISInputSource]
for source in list {
    guard let raw = TISGetInputSourceProperty(source, kTISPropertyInputSourceID) else { continue }
    let id = Unmanaged<CFString>.fromOpaque(raw).takeUnretainedValue() as String
    if id == "com.apple.keylayout.ABC" {
        TISSelectInputSource(source)
        fputs("ok\n", stderr)
        exit(0)
    }
}
fputs("abc-not-found\n", stderr)
exit(1)
