/**
 * EventBusContext - React Context for EventBus Injection
 *
 * Provides the EventBus instance to components via React context,
 * enabling hooks like useEvent to use the shared EventBus instance
 * instead of creating separate Tauri listeners.
 */

import { createContext, useContext, ReactNode } from "react";
import type { IEventBus } from "../types/kernel";

// Create context with undefined default (must be provided)
const EventBusContext = createContext<IEventBus | undefined>(undefined);

export interface EventBusProviderProps {
  eventBus: IEventBus;
  children: ReactNode;
}

/**
 * Provider component that makes the EventBus available to descendants.
 *
 * @example
 * ```tsx
 * <EventBusProvider eventBus={eventBus}>
 *   <App />
 * </EventBusProvider>
 * ```
 */
export function EventBusProvider({ eventBus, children }: EventBusProviderProps) {
  return (
    <EventBusContext.Provider value={eventBus}>
      {children}
    </EventBusContext.Provider>
  );
}

/**
 * Hook to access the EventBus from context.
 * Throws if used outside of an EventBusProvider.
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const eventBus = useEventBus();
 *
 *   useEffect(() => {
 *     const unsub = eventBus.subscribe("session.*", (event) => {
 *       console.log("Session event:", event);
 *     });
 *     return unsub;
 *   }, [eventBus]);
 *
 *   return <div>...</div>;
 * }
 * ```
 */
export function useEventBus(): IEventBus {
  const eventBus = useContext(EventBusContext);
  if (!eventBus) {
    throw new Error(
      "useEventBus must be used within an EventBusProvider. " +
      "Make sure to wrap your app with <EventBusProvider>."
    );
  }
  return eventBus;
}

/**
 * Hook to optionally access the EventBus from context.
 * Returns undefined if used outside of an EventBusProvider.
 * Useful for components that may work with or without the context.
 */
export function useEventBusOptional(): IEventBus | undefined {
  return useContext(EventBusContext);
}

export { EventBusContext };
