import { ComponentRegistry } from "../kernel/ComponentRegistry";
import { SlotName } from "../types/slots";

interface SlotRendererProps {
  registry: ComponentRegistry;
  slot: SlotName;
}

export function SlotRenderer({ registry, slot }: SlotRendererProps) {
  const components = registry.getComponents(slot);

  if (components.length === 0) return null;

  return (
    <>
      {components.map(({ id, component: Component }) => (
        <Component key={id} />
      ))}
    </>
  );
}
