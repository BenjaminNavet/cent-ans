// Prints "<CGWindowID> x,y,w,h" of the largest on-screen layer-0 window owned by the given process name.
import CoreGraphics
import Foundation
let owner = CommandLine.arguments.dropFirst().first ?? "cent-ans"
let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
var best: (Double, String)?
for w in list where (w[kCGWindowOwnerName as String] as? String) == owner && (w[kCGWindowLayer as String] as? Int) == 0 {
    let b = w[kCGWindowBounds as String] as? [String: Double] ?? [:]
    let area = (b["Width"] ?? 0) * (b["Height"] ?? 0)
    if area > (best?.0 ?? 0) {
        best = (area, "\(w[kCGWindowNumber as String] as? Int ?? 0) \(Int(b["X"] ?? 0)),\(Int(b["Y"] ?? 0)),\(Int(b["Width"] ?? 0)),\(Int(b["Height"] ?? 0))")
    }
}
if let best { print(best.1) }
