// 맥 자동 입력 시험 도구(손쉬운 사용 권한 필요) — 단계를 차례로 보낸다. 좌표는 CG 전역 포인트.
//   swift bench/mac/input.swift "move 960 540" "key opt+1" "wait 0.6" "drag 800 400 1100 500" "key esc"
// 단계: move x y · dclick x y · key [cmd+|opt+|shift+|ctrl+]이름 · drag x0 y0 x1 y1 [걸음] · click x y · rclick x y
//       · down x y · to x y [걸음](누른 채 옮기기) · up · circle cx cy r(누른 채 한 바퀴, 놓지 않음)
//       · scroll 칸(+ = 위, 커서 자리) · type 글자(ASCII) · wait 초
// 키 이름: a~z · 0~9 · esc · enter · space · tab · delete · f1~f4 · up · down · left · right
// osascript System Events는 자동화 권한 창이 따로 떠서 쓰지 않는다(인수인계 2026-11-25 §4).
import CoreGraphics
import Foundation

let src = CGEventSource(stateID: .hidSystemState)
let named: [String: CGKeyCode] = [
  "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9, "b": 11, "q": 12, "w": 13,
  "e": 14, "r": 15, "y": 16, "t": 17, "1": 18, "2": 19, "3": 20, "4": 21, "6": 22, "5": 23, "9": 25, "7": 26,
  "8": 28, "0": 29, "o": 31, "u": 32, "i": 34, "p": 35, "l": 37, "j": 38, "k": 40, "n": 45, "m": 46,
  "enter": 36, "tab": 48, "space": 49, "delete": 51, "esc": 53,
  "f1": 122, "f2": 120, "f3": 99, "f4": 118, "left": 123, "right": 124, "down": 125, "up": 126,
]
let mods: [String: (CGEventFlags, CGKeyCode)] = [
  "cmd": (.maskCommand, 55), "shift": (.maskShift, 56), "opt": (.maskAlternate, 58), "ctrl": (.maskControl, 59),
]
var cursor = CGEvent(source: nil)?.location ?? .zero

func post(_ e: CGEvent?) { e?.post(tap: .cghidEventTap) }

func mouse(_ t: CGEventType, _ p: CGPoint, _ b: CGMouseButton = .left) {
  cursor = p
  post(CGEvent(mouseEventSource: src, mouseType: t, mouseCursorPosition: p, mouseButton: b))
}

func key(_ spec: String) {
  var parts = spec.lowercased().split(separator: "+").map(String.init)
  guard let name = parts.popLast(), let code = named[name] else { fatalError("모르는 키: \(spec)") }
  var flags: CGEventFlags = []
  for m in parts {
    guard let (f, k) = mods[m] else { fatalError("모르는 보조 키: \(m)") }
    flags.insert(f)
    let e = CGEvent(keyboardEventSource: src, virtualKey: k, keyDown: true); e?.flags = flags; post(e); usleep(15_000)
  }
  for down in [true, false] {
    // 기본 플래그(F 키의 fn 표시 등)는 두고 보조 키만 더한다 — 빈 값으로 덮으면 F1 전역 단축키가 안 먹는다
    let e = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down)
    if let e { e.flags = e.flags.union(flags) }
    post(e); usleep(20_000)
  }
  for m in parts.reversed() {
    let (f, k) = mods[m]!
    flags.remove(f)
    let e = CGEvent(keyboardEventSource: src, virtualKey: k, keyDown: false); e?.flags = flags; post(e); usleep(15_000)
  }
}

func type(_ text: String) {
  for ch in text {
    var u = Array(String(ch).utf16)
    for down in [true, false] {
      let e = CGEvent(keyboardEventSource: src, virtualKey: 0, keyDown: down)
      e?.keyboardSetUnicodeString(stringLength: u.count, unicodeString: &u); post(e); usleep(25_000)
    }
  }
}

for step in CommandLine.arguments.dropFirst() {
  let w = step.split(separator: " ").map(String.init)
  let n = w.dropFirst().compactMap { Double($0) }
  switch w.first {
  case "move": mouse(.mouseMoved, CGPoint(x: n[0], y: n[1])); usleep(60_000)
  case "key": key(w[1])
  case "type": type(w.dropFirst().joined(separator: " "))
  case "wait": usleep(useconds_t(n[0] * 1_000_000))
  case "click", "rclick":
    let p = CGPoint(x: n[0], y: n[1]), r = w[0] == "rclick"
    mouse(.mouseMoved, p); usleep(60_000)
    mouse(r ? .rightMouseDown : .leftMouseDown, p, r ? .right : .left); usleep(60_000)
    mouse(r ? .rightMouseUp : .leftMouseUp, p, r ? .right : .left); usleep(60_000)
  case "drag":
    let (a, b) = (CGPoint(x: n[0], y: n[1]), CGPoint(x: n[2], y: n[3]))
    let steps = n.count > 4 ? Int(n[4]) : 24
    mouse(.mouseMoved, a); usleep(60_000)
    mouse(.leftMouseDown, a); usleep(50_000)
    for i in 1...steps {
      let t = Double(i) / Double(steps)
      mouse(.leftMouseDragged, CGPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t)); usleep(16_000)
    }
    usleep(40_000); mouse(.leftMouseUp, b); usleep(80_000)
  case "dclick":
    let p = CGPoint(x: n[0], y: n[1])
    mouse(.mouseMoved, p); usleep(60_000)
    for state in [1, 2] as [Int64] {
      for t in [CGEventType.leftMouseDown, .leftMouseUp] {
        let e = CGEvent(mouseEventSource: src, mouseType: t, mouseCursorPosition: p, mouseButton: .left)
        e?.setIntegerValueField(.mouseEventClickState, value: state); post(e); usleep(30_000)
      }
    }
    usleep(60_000)
  case "down": mouse(.mouseMoved, CGPoint(x: n[0], y: n[1])); usleep(60_000); mouse(.leftMouseDown, cursor); usleep(50_000)
  case "to":
    let (a, b) = (cursor, CGPoint(x: n[0], y: n[1]))
    let steps = n.count > 2 ? Int(n[2]) : 12
    for i in 1...steps {
      let t = Double(i) / Double(steps)
      mouse(.leftMouseDragged, CGPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t)); usleep(16_000)
    }
  case "up": mouse(.leftMouseUp, cursor); usleep(80_000)
  case "circle":
    for i in 0...48 {
      let t = Double(i) / 48 * 2 * Double.pi
      mouse(.leftMouseDragged, CGPoint(x: n[0] + n[2] * cos(t), y: n[1] + n[2] * sin(t))); usleep(16_000)
    }
  case "scroll":
    post(CGEvent(scrollWheelEvent2Source: src, units: .line, wheelCount: 1, wheel1: Int32(n[0]), wheel2: 0, wheel3: 0)); usleep(80_000)
  default: fatalError("모르는 단계: \(step)")
  }
}
