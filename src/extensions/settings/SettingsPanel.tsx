import React, { useState, useEffect, useRef } from "react";
import type { IKeybindingManager, KeybindingDefinition } from "../../types/kernel";
import { useConfig } from "../../kernel/ConfigProvider";

let settingsBindings: KeybindingDefinition[] = [];
let showSettings = false;
let settingsListeners: Array<() => void> = [];
let keybindingManagerInstance: IKeybindingManager | null = null;

export function notifySettings() {
  settingsListeners.forEach((l) => l());
}

export function setSettingsState(
  bindings: KeybindingDefinition[],
  visible: boolean,
  manager?: IKeybindingManager
) {
  settingsBindings = bindings;
  showSettings = visible;
  if (manager) keybindingManagerInstance = manager;
  notifySettings();
}

export function getSettingsVisible() {
  return showSettings;
}

function Toggle({
  checked,
  onChange,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      className={`settings-toggle ${checked ? "on" : ""} ${disabled ? "disabled" : ""}`}
      onClick={() => !disabled && onChange(!checked)}
      disabled={disabled}
      role="switch"
      aria-checked={checked}
    >
      <span className="settings-toggle-thumb" />
    </button>
  );
}

function NumberInput({
  value,
  onChange,
  min,
  max,
  step,
  suffix,
}: {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  suffix?: string;
}) {
  return (
    <div className="settings-number-input">
      <input
        type="number"
        value={value}
        onChange={(e) => {
          const val = parseInt(e.target.value, 10);
          if (!isNaN(val)) {
            const clamped = Math.max(min ?? 1, Math.min(max ?? 60, val));
            onChange(clamped);
          }
        }}
        min={min}
        max={max}
        step={step}
      />
      {suffix && <span className="settings-number-suffix">{suffix}</span>}
    </div>
  );
}

function KeyRecorder({
  binding,
  manager,
  allBindings,
  onDone,
}: {
  binding: KeybindingDefinition;
  manager: IKeybindingManager;
  allBindings: KeybindingDefinition[];
  onDone: () => void;
}) {
  const [recording, setRecording] = useState(false);
  const [conflict, setConflict] = useState<string | null>(null);
  const ref = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    if (!recording) return;
    manager.setActive(false);

    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Ignore events that are only modifier keys (wait for the actual key)
      if (["Meta", "Control", "Alt", "Shift"].includes(e.key)) {
        return;
      }

      const keyStr = manager.eventToKeyString(e);
      if (!keyStr || keyStr === "Escape") {
        setRecording(false);
        manager.setActive(true);
        return;
      }
      // Check for conflicts
      const conflicting = allBindings.find(
        (b) => b.id !== binding.id && b.keys === keyStr
      );
      if (conflicting) {
        setConflict(`Conflicts with "${conflicting.label}"`);
        setTimeout(() => setConflict(null), 2000);
        return;
      }
      manager.updateKeys(binding.id, keyStr);
      manager.setActive(true);
      setRecording(false);
      onDone();
    };

    window.addEventListener("keydown", handler, true);
    return () => {
      window.removeEventListener("keydown", handler, true);
      manager.setActive(true);
    };
  }, [recording, binding, manager, allBindings, onDone]);

  const isCustom = binding.defaultKeys && binding.keys !== binding.defaultKeys;

  return (
    <span className="settings-keybinding-controls">
      <span
        ref={ref}
        className={`settings-keybinding-keys ${recording ? "recording" : ""} ${isCustom ? "custom" : ""}`}
        onClick={(e) => {
          e.stopPropagation();
          setRecording(true);
        }}
        title="Click to record new shortcut"
      >
        {recording ? "Press keys..." : binding.keys}
      </span>
      {isCustom && (
        <button
          className="settings-keybinding-reset"
          onClick={(e) => {
            e.stopPropagation();
            manager.resetKeys(binding.id);
            onDone();
          }}
          title="Reset to default"
        >
          ↺
        </button>
      )}
      {conflict && <span className="settings-keybinding-conflict">{conflict}</span>}
    </span>
  );
}

export function SettingsPanel() {
  const [visible, setVisible] = useState(false);
  const [bindings, setBindings] = useState<KeybindingDefinition[]>([]);
  const [, setRefresh] = useState(0);
  const config = useConfig();

  // Auto-focus settings
  const windowAutoFocus = config.get<boolean>("autoFocus.windowBringToFront", true);
  const aggressiveMode = config.get<boolean>("autoFocus.aggressiveMode", false);
  const tabAutoSwitch = config.get<boolean>("autoFocus.tabAutoSwitch", true);
  const inactivitySeconds = config.get<number>("autoFocus.inactivitySeconds", 5);
  const countdownSeconds = config.get<number>("autoFocus.countdownSeconds", 3);

  // Session idle timeout (minutes before session transitions to Idle state)
  const idleTimeoutMinutes = config.get<number>("session.idleTimeoutMinutes", 30);

  useEffect(() => {
    const update = () => {
      const wasVisible = visible;
      setVisible(showSettings);
      setBindings([...settingsBindings]);
      if (!showSettings && wasVisible) {
        // Return focus to terminal when settings closes
        const terminal = document.querySelector(".xterm-helper-textarea") as HTMLTextAreaElement;
        if (terminal) terminal.focus();
      }
    };
    settingsListeners.push(update);
    return () => {
      settingsListeners = settingsListeners.filter((l) => l !== update);
    };
  }, [visible]);

  if (!visible) return null;

  const close = () => {
    showSettings = false;
    notifySettings();
  };

  const handleDone = () => {
    if (keybindingManagerInstance) {
      setBindings([...keybindingManagerInstance.getAll()]);
    }
    setRefresh((n) => n + 1);
  };

  // Group bindings by extension
  const grouped = bindings.reduce<Record<string, KeybindingDefinition[]>>((acc, b) => {
    const key = b.extensionId;
    if (!acc[key]) acc[key] = [];
    acc[key].push(b);
    return acc;
  }, {});

  // Format extension ID to display name (e.g., "tab-bar" -> "Tab Bar")
  const formatExtensionName = (id: string): string => {
    return id
      .split("-")
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(" ");
  };

  return (
    <div className="settings-backdrop" onClick={close} role="presentation">
      <div className="settings-panel" onClick={(e) => e.stopPropagation()} role="dialog" aria-label="Settings" aria-modal="true">
        <div className="settings-panel-header">
          <span className="settings-panel-title">Settings</span>
          <button className="settings-panel-close" onClick={close} aria-label="Close settings">
            &times;
          </button>
        </div>
        <div className="settings-panel-list">
          {/* General Settings Section */}
          <div className="settings-panel-section">General</div>

          <div className="settings-item">
            <div className="settings-item-info">
              <span className="settings-item-label">Auto-focus window on Your Turn</span>
              <span className="settings-item-desc">
                Bring window to front when a session needs your attention
              </span>
            </div>
            <Toggle
              checked={windowAutoFocus}
              onChange={(checked) => config.set("autoFocus.windowBringToFront", checked)}
            />
          </div>

          <div className="settings-item">
            <div className="settings-item-info">
              <span className="settings-item-label">Aggressive focus mode</span>
              <span className="settings-item-desc">
                Force window to front even when other apps are focused (uses always-on-top)
              </span>
            </div>
            <Toggle
              checked={aggressiveMode}
              onChange={(checked) => config.set("autoFocus.aggressiveMode", checked)}
              disabled={!windowAutoFocus}
            />
          </div>

          <div className="settings-item">
            <div className="settings-item-info">
              <span className="settings-item-label">Auto-switch to Your Turn tabs</span>
              <span className="settings-item-desc">
                Switch to sessions needing attention after inactivity
              </span>
            </div>
            <Toggle
              checked={tabAutoSwitch}
              onChange={(checked) => config.set("autoFocus.tabAutoSwitch", checked)}
            />
          </div>

          <div className="settings-item">
            <div className="settings-item-info">
              <span className="settings-item-label">Inactivity timeout</span>
              <span className="settings-item-desc">
                Seconds of inactivity before showing switch prompt
              </span>
            </div>
            <NumberInput
              value={inactivitySeconds}
              onChange={(val) => config.set("autoFocus.inactivitySeconds", val)}
              min={1}
              max={60}
              suffix="s"
            />
          </div>

          <div className="settings-item">
            <div className="settings-item-info">
              <span className="settings-item-label">Countdown duration</span>
              <span className="settings-item-desc">
                Seconds to count down before auto-switching
              </span>
            </div>
            <NumberInput
              value={countdownSeconds}
              onChange={(val) => config.set("autoFocus.countdownSeconds", val)}
              min={1}
              max={10}
              suffix="s"
            />
          </div>

          <div className="settings-item">
            <div className="settings-item-info">
              <span className="settings-item-label">Idle timeout</span>
              <span className="settings-item-desc">
                Minutes of inactivity before a session transitions to Idle state
              </span>
            </div>
            <NumberInput
              value={idleTimeoutMinutes}
              onChange={(val) => config.set("session.idleTimeoutMinutes", val)}
              min={1}
              max={120}
              suffix="min"
            />
          </div>

          {/* Keyboard Shortcuts Section */}
          {Object.entries(grouped).map(([extId, items]) => (
            <React.Fragment key={extId}>
              <div className="settings-panel-section">{formatExtensionName(extId)}</div>
              {items.map((b) => (
                <div key={b.id} className="settings-keybinding">
                  <span className="settings-keybinding-label">{b.label}</span>
                  {keybindingManagerInstance ? (
                    <KeyRecorder
                      binding={b}
                      manager={keybindingManagerInstance}
                      allBindings={bindings}
                      onDone={handleDone}
                    />
                  ) : (
                    <span className="settings-keybinding-keys">{b.keys}</span>
                  )}
                </div>
              ))}
            </React.Fragment>
          ))}
        </div>
      </div>
    </div>
  );
}
