// 맥 자동 입력 시험 도구(손쉬운 사용 권한 필요). 진입 시간 측정: bench/m1-overlay-mac.sh
import CoreGraphics
import Foundation
// 사용법: keys <횟수> <누름 뒤 대기 초> <나간 뒤 대기 초>
let a = CommandLine.arguments
let n = Int(a[1]) ?? 10, hold = Double(a[2]) ?? 0.35, gap = Double(a[3]) ?? 0.35
func tap(_ k: CGKeyCode) {
  let src = CGEventSource(stateID: .hidSystemState)
  CGEvent(keyboardEventSource: src, virtualKey: k, keyDown: true)?.post(tap: .cghidEventTap)
  usleep(20_000)
  CGEvent(keyboardEventSource: src, virtualKey: k, keyDown: false)?.post(tap: .cghidEventTap)
}
for _ in 0..<n { tap(122); Thread.sleep(forTimeInterval: hold); tap(53); Thread.sleep(forTimeInterval: gap) }
