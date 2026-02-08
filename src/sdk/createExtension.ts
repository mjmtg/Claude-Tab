import { FrontendExtension, ExtensionManifest, ExtensionContext } from "../types/extension";

interface ExtensionOptions {
  manifest: ExtensionManifest;
  activate: (ctx: ExtensionContext) => void | Promise<void>;
  deactivate?: () => void | Promise<void>;
}

export function createExtension(options: ExtensionOptions): FrontendExtension {
  return {
    manifest: options.manifest,
    activate: options.activate,
    deactivate: options.deactivate,
  };
}
