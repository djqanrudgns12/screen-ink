// 맥 자동 입력 시험 도구(손쉬운 사용 권한 필요). 모니터를 넘어 끌기 확인: swift bench/mac/drag.swift 1500 500 2020 300
import CoreGraphics
import Foundation
// 사용법: drag x0 y0 x1 y1 (CG 전역 포인트). F1 → 커서 이동 → 눌러 끌기 → 놓기 → Esc
let a = CommandLine.arguments.dropFirst().compactMap { Double($0) }
let (x0, y0, x1, y1) = (a[0], a[1], a[2], a[3])
let src = CGEventSource(stateID: .hidSystemState)
func key(_ k: CGKeyCode) {
  CGEvent(keyboardEventSource: src, virtualKey: k, keyDown: true)?.post(tap: .cghidEventTap); usleep(20_000)
  CGEvent(keyboardEventSource: src, virtualKey: k, keyDown: false)?.post(tap: .cghidEventTap)
}
func mouse(_ t: CGEventType, _ x: Double, _ y: Double) {
  CGEvent(mouseEventSource: src, mouseType: t, mouseCursorPosition: CGPoint(x: x, y: y), mouseButton: .left)?.post(tap: .cghidEventTap)
}
mouse(.mouseMoved, x0, y0); usleep(150_000)
key(122); usleep(500_000)
mouse(.mouseMoved, x0 + 6, y0 + 6); usleep(100_000); mouse(.mouseMoved, x0, y0); usleep(100_000)
mouse(.leftMouseDown, x0, y0); usleep(80_000)
for i in 1...20 { let t = Double(i) / 20; mouse(.leftMouseDragged, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t); usleep(25_000) }
usleep(200_000)
mouse(.leftMouseUp, x1, y1); usleep(400_000)
key(53); usleep(300_000)
