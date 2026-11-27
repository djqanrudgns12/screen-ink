// 단축키 추천 묶음 3종(R2-KEY-6): [발표용 기본] · [Snipaste 습관] · [한 손으로(왼손)]
import presetsData from './presets.json' with { type: 'json' };
import { normalizeOs } from '../i18n/core.js';

/**
 * @typedef {'presentation' | 'snipaste' | 'lefthand'} PresetId
 * @typedef {{ id: PresetId, titleKey: string, descKey: string, badgeKey?: string, win: Record<string, string>, mac: Record<string, string> }} ShortcutPreset
 */

/** @type {ReadonlyArray<ShortcutPreset>} */
export const SHORTCUT_PRESETS = Object.freeze(/** @type {any} */ (presetsData).presets);

/**
 * 묶음 id와 OS에 맞는 슬롯별 단축키 맵을 돌려준다.
 * @param {string} presetId
 * @param {'windows' | 'macos' | string} [platform='windows']
 * @returns {Record<string, string>}
 */
export function getPresetCombos(presetId, platform = 'windows') {
  const preset = SHORTCUT_PRESETS.find((p) => p.id === presetId);
  if (!preset) return {};
  const isMac = normalizeOs(platform) === 'mac';
  return { ...(isMac ? preset.mac : preset.win) };
}

/**
 * 현재 단축키 목록과 정확히 일치하는 추천 묶음 id를 찾는다.
 * @param {ReadonlyArray<{ id: string, combo: string }>} shortcuts
 * @param {'windows' | 'macos' | string} [platform='windows']
 * @returns {PresetId | null}
 */
export function detectActivePreset(shortcuts, platform = 'windows') {
  if (!Array.isArray(shortcuts) || shortcuts.length === 0) return null;
  const byId = new Map(shortcuts.map((s) => [s.id, s.combo || '']));
  for (const preset of SHORTCUT_PRESETS) {
    const target = getPresetCombos(preset.id, platform);
    const entries = Object.entries(target);
    if (entries.length === 0) continue;
    const allMatch = entries.every(([id, combo]) => {
      if (!byId.has(id)) return true;
      return (byId.get(id) ?? '') === combo;
    });
    if (allMatch) return preset.id;
  }
  return null;
}

/**
 * 선택한 묶음을 적용했을 때 바뀔 단축키 목록만 뽑는다.
 * @param {string} presetId
 * @param {ReadonlyArray<{ id: string, label?: string, combo: string }>} shortcuts
 * @param {'windows' | 'macos' | string} [platform='windows']
 * @returns {Array<{ id: string, label: string, before: string, after: string }>}
 */
export function diffPreset(presetId, shortcuts, platform = 'windows') {
  const target = getPresetCombos(presetId, platform);
  const byId = new Map((shortcuts ?? []).map((s) => [s.id, s]));
  /** @type {Array<{ id: string, label: string, before: string, after: string }>} */
  const changes = [];
  for (const [id, after] of Object.entries(target)) {
    const row = byId.get(id);
    if (!row) continue;
    const before = row.combo || '';
    if (before === after) continue;
    changes.push({ id, label: row.label || id, before, after });
  }
  return changes;
}

/**
 * 묶음 미리보기 중 다른 슬롯과 내부 자리바꿈으로 일시 겹치는 경우를 걸러낸다.
 * (예: record가 F3→F4로 비워지고 pin이 Alt+Shift+V→F3으로 들어가는 경우)
 * 외부 앱(owner 있음)이나 OS 예약 겹침은 그대로 유지한다.
 * @param {{ id: string, before: string, after: string }} entry
 * @param {{ status?: string, reason?: string, owner?: string }} previewRes
 * @param {ReadonlyArray<{ id: string, before: string, after: string }>} diffList
 * @param {ReadonlyArray<{ id: string, combo: string }>} shortcuts
 * @returns {{ status: 'ok' | 'warn' | 'conflict' | 'off', reason: string, owner: string }}
 */
export function resolvePreviewStatus(entry, previewRes, diffList, shortcuts) {
  const status = /** @type {any} */ (previewRes?.status ?? 'ok');
  const reason = previewRes?.reason || '';
  const owner = previewRes?.owner || '';

  const isSelfOrEmptyOwner = !owner || owner === '슉쇽' || owner === 'ShookShok';
  if (status === 'conflict' && isSelfOrEmptyOwner) {
    const holder = (shortcuts ?? []).find((s) => s.id !== entry.id && s.combo === entry.after);
    if (holder) {
      const holderDiff = diffList.find((d) => d.id === holder.id);
      if (holderDiff && holderDiff.after !== entry.after) {
        return { status: 'ok', reason: '', owner: '' };
      }
    }
  }
  return { status, reason, owner };
}

/**
 * 내부 자리바꿈·순환(예: A→B, B→A 또는 record:F3→F4, pin:X→F3)이
 * setShortcut 순차 적용 중 임시 충돌을 일으키지 않도록 적용 순서를 계산한다.
 * @param {ReadonlyArray<{ id: string, before: string, after: string }>} diffList
 * @returns {Array<{ id: string, combo: string, isTempClear?: boolean }>}
 */
export function planPresetApplyOrder(diffList) {
  const pending = diffList.map((d) => ({ ...d }));
  /** @type {Array<{ id: string, combo: string, isTempClear?: boolean }>} */
  const steps = [];
  /** @type {Set<string>} */
  const tempCleared = new Set();

  while (pending.length > 0) {
    const occupied = new Set(pending.map((d) => d.before).filter(Boolean));
    const readyIdx = pending.findIndex((d) => !occupied.has(d.after));
    if (readyIdx !== -1) {
      const [picked] = pending.splice(readyIdx, 1);
      steps.push({ id: picked.id, combo: picked.after });
      continue;
    }

    // 순환(A→B, B→A)이 남았을 때 하나를 먼저 비워 자리를 연다.
    const breakIdx = pending.findIndex((d) => !tempCleared.has(d.id) && Boolean(d.before));
    if (breakIdx === -1) {
      const [picked] = pending.splice(0, 1);
      steps.push({ id: picked.id, combo: picked.after });
      continue;
    }
    const target = pending[breakIdx];
    tempCleared.add(target.id);
    steps.push({ id: target.id, combo: '', isTempClear: true });
    target.before = '';
  }

  return steps;
}

/**
 * 클라이언트 `previewShortcut`을 이용해 바뀔 키 목록과 겹침 여부를 계산한다.
 * @param {import('@screen-ink/client').InkClient} client
 * @param {string} presetId
 * @param {ReadonlyArray<{ id: string, label?: string, combo: string }>} shortcuts
 * @param {'windows' | 'macos' | string} [platform='windows']
 */
export async function previewPresetWithClient(client, presetId, shortcuts, platform = 'windows') {
  const changes = diffPreset(presetId, shortcuts, platform);
  const items = await Promise.all(
    changes.map(async (entry) => {
      let res = { status: 'ok', reason: '', owner: '' };
      if (typeof client?.previewShortcut === 'function') {
        try {
          const raw = await client.previewShortcut(entry.id, entry.after);
          res = resolvePreviewStatus(entry, raw, changes, shortcuts);
        } catch {
          res = { status: 'ok', reason: '', owner: '' };
        }
      }
      return {
        ...entry,
        status: res.status,
        reason: res.reason,
        owner: res.owner,
      };
    }),
  );

  const conflictCount = items.filter((i) => i.status === 'conflict').length;
  const warnCount = items.filter((i) => i.status === 'warn').length;
  return { presetId, changes: items, conflictCount, warnCount };
}

/**
 * 선택한 추천 묶음을 기존 `client.setShortcut`으로 순서대로 적용한다.
 * 반환값의 `undoSteps`를 `undoPresetApply`에 넘기면 적용 전 상태로 되돌릴 수 있다.
 * @param {import('@screen-ink/client').InkClient} client
 * @param {string} presetId
 * @param {ReadonlyArray<{ id: string, label?: string, combo: string }>} shortcuts
 * @param {'windows' | 'macos' | string} [platform='windows']
 * @returns {Promise<{ ok: boolean, undoSteps: number, appliedCount: number, failed: Array<{ id: string, combo: string, reason: string }> }>}
 */
export async function applyPreset(client, presetId, shortcuts, platform = 'windows') {
  const diff = diffPreset(presetId, shortcuts, platform);
  const steps = planPresetApplyOrder(diff);
  let undoSteps = 0;
  let appliedCount = 0;
  /** @type {Array<{ id: string, combo: string, reason: string }>} */
  const failed = [];

  for (const step of steps) {
    const res = await client.setShortcut(step.id, step.combo);
    if (res?.ok) {
      undoSteps += 1;
      if (!step.isTempClear) appliedCount += 1;
    } else if (!step.isTempClear) {
      failed.push({ id: step.id, combo: step.combo, reason: res?.reason || '' });
    }
  }

  return {
    ok: failed.length === 0,
    undoSteps,
    appliedCount,
    failed,
  };
}

/**
 * 직전 추천 묶음 적용을 기존 `client.undo()` 스택으로 되돌린다.
 * @param {import('@screen-ink/client').InkClient} client
 * @param {number} undoSteps
 * @returns {Promise<boolean>}
 */
export async function undoPresetApply(client, undoSteps) {
  let ok = true;
  for (let i = 0; i < undoSteps; i += 1) {
    const res = await client.undo();
    if (!res?.ok) {
      ok = false;
      break;
    }
  }
  return ok;
}
