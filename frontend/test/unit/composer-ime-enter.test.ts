// @vitest-environment jsdom

import { act, createElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MessageComposer } from "@/views/room/MessageComposer";

/**
 * An Enter that confirms an IME candidate (or an emoji picker's composition)
 * is not a send: `isComposing` is still true on that keydown. The composer
 * must leave the draft alone for it and send only on a settled Enter.
 */

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

function composerWith(onSend: (text: string) => void) {
  act(() => {
    root.render(createElement(MessageComposer, { placeholder: "Message", onSend }));
  });
  return container.querySelector("textarea")!;
}

// Setting `.value` directly only moves the DOM node; React's value tracker
// then sees no change and swallows the input event. Go through the prototype
// setter so the controlled component actually receives the edit.
function setValue(target: HTMLTextAreaElement, value: string) {
  Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(target, value);
  target.dispatchEvent(new Event("input", { bubbles: true }));
}

function enter(target: HTMLTextAreaElement, isComposing: boolean) {
  act(() => {
    target.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true, isComposing }),
    );
  });
}

describe("MessageComposer and IME composition", () => {
  it("does not send while a composition is being confirmed", () => {
    const onSend = vi.fn();
    const textarea = composerWith(onSend);
    act(() => {
      setValue(textarea, "かな");
    });
    enter(textarea, true);
    expect(onSend).not.toHaveBeenCalled();
    expect(textarea.value).toBe("かな");
  });

  it("still sends on a settled Enter", () => {
    const onSend = vi.fn();
    const textarea = composerWith(onSend);
    act(() => {
      setValue(textarea, "hello");
    });
    enter(textarea, false);
    expect(onSend).toHaveBeenCalledWith("hello", undefined, undefined, undefined);
  });
});
