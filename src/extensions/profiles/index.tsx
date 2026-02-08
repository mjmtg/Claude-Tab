import { FrontendExtension } from "../../types/extension";
import { SLOTS } from "../../types/slots";
import { ProfilesPanel, setProfilesPanelVisible, getProfilesPanelVisible } from "./ProfilesPanel";

export function toggleProfiles() {
  setProfilesPanelVisible(!getProfilesPanelVisible());
}

export function createProfilesExtension(): FrontendExtension {
  return {
    manifest: {
      id: "profiles",
      name: "Profiles",
      version: "0.1.0",
      description: "Profile-based session templates",
      dependencies: ["tab-bar"],
    },
    activate(ctx) {
      ctx.componentRegistry.register(SLOTS.OVERLAY, {
        id: "profiles-overlay",
        component: ProfilesPanel,
        priority: 80,
        extensionId: "profiles",
      });

      ctx.keybindingManager.register({
        id: "profiles.toggle",
        keys: "Cmd+Shift+P",
        label: "Profiles",
        extensionId: "profiles",
        handler: toggleProfiles,
      });
    },
  };
}
