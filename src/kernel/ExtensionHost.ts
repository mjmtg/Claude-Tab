import { FrontendExtension, ExtensionContext } from "../types/extension";
import type {
  IEventBus,
  IComponentRegistry,
  IKeybindingManager,
  IFocusManager,
  ISessionStateManager,
} from "../types/kernel";
import type { IBackendService, IEventListener } from "../services/types";

export interface ExtensionHostConfig {
  eventBus: IEventBus;
  componentRegistry: IComponentRegistry;
  keybindingManager: IKeybindingManager;
  backend: IBackendService;
  eventListener: IEventListener;
  focusManager: IFocusManager;
  sessionStateManager: ISessionStateManager;
}

export class ExtensionHost {
  private extensions: FrontendExtension[] = [];
  private activated: string[] = [];
  private ctx: ExtensionContext;

  constructor(config: ExtensionHostConfig) {
    this.ctx = {
      eventBus: config.eventBus,
      componentRegistry: config.componentRegistry,
      keybindingManager: config.keybindingManager,
      backend: config.backend,
      eventListener: config.eventListener,
      focusManager: config.focusManager,
      sessionStateManager: config.sessionStateManager,
    };
  }

  register(extension: FrontendExtension): void {
    this.extensions.push(extension);
  }

  async activateAll(): Promise<void> {
    const order = this.resolveOrder();
    for (const ext of order) {
      try {
        await ext.activate(this.ctx);
        this.activated.push(ext.manifest.id);
        console.log(`[ExtensionHost] Activated: ${ext.manifest.id}`);
      } catch (err) {
        console.error(`[ExtensionHost] Failed to activate ${ext.manifest.id}:`, err);
      }
    }
  }

  async deactivateAll(): Promise<void> {
    for (const ext of [...this.extensions].reverse()) {
      if (this.activated.includes(ext.manifest.id) && ext.deactivate) {
        try {
          await ext.deactivate();
        } catch (err) {
          console.error(`[ExtensionHost] Failed to deactivate ${ext.manifest.id}:`, err);
        }
      }
    }
    this.activated = [];
  }

  private resolveOrder(): FrontendExtension[] {
    const idMap = new Map<string, FrontendExtension>();
    for (const ext of this.extensions) {
      idMap.set(ext.manifest.id, ext);
    }

    const visited = new Set<string>();
    const order: FrontendExtension[] = [];

    const visit = (ext: FrontendExtension) => {
      if (visited.has(ext.manifest.id)) return;
      visited.add(ext.manifest.id);
      for (const dep of ext.manifest.dependencies ?? []) {
        const depExt = idMap.get(dep);
        if (depExt) visit(depExt);
      }
      order.push(ext);
    };

    for (const ext of this.extensions) {
      visit(ext);
    }

    return order;
  }
}
