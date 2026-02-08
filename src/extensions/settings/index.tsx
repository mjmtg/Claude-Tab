import { FrontendExtension } from "../../types/extension";
import type { IKeybindingManager } from "../../types/kernel";
import { SLOTS } from "../../types/slots";
import { SettingsPanel, setSettingsState, getSettingsVisible } from "./SettingsPanel";

let keybindingManagerRef: IKeybindingManager | null = null;

export function toggleSettings() {
  if (!keybindingManagerRef) return;
  const bindings = keybindingManagerRef.getAll();
  setSettingsState(bindings, !getSettingsVisible(), keybindingManagerRef);
}

export function createSettingsExtension(): FrontendExtension {
  return {
    manifest: {
      id: "settings",
      name: "Settings",
      version: "0.1.0",
      description: "Keyboard shortcuts viewer (Cmd+,)",
    },
    activate(ctx) {
      keybindingManagerRef = ctx.keybindingManager;

      ctx.componentRegistry.register(SLOTS.OVERLAY, {
        id: "settings-overlay",
        component: SettingsPanel,
        priority: 90,
        extensionId: "settings",
      });

      ctx.keybindingManager.register({
        id: "settings.toggle",
        keys: "Cmd+,",
        label: "Settings",
        extensionId: "settings",
        handler: toggleSettings,
      });
    },
  };
}
