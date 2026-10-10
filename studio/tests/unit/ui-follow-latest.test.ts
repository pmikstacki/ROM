import test from "node:test";
import assert from "node:assert/strict";
import { attachFollowLatest } from "../../src/lib/ui/follow-latest.ts";

function fixture() {
  let frame: FrameRequestCallback | null = null;
  let observed = false;
  let disconnected = false;
  let next = 0;
  const frames = new Map<number, FrameRequestCallback>();
  class Observer {
    constructor(callback: ResizeObserverCallback) {
      frame = () => callback([], this as unknown as ResizeObserver);
    }
    observe() {
      observed = true;
    }
    disconnect() {
      disconnected = true;
    }
  }
  const element = Object.assign(new EventTarget(), {
    scrollHeight: 1000,
    clientHeight: 200,
    scrollTop: 800,
    ownerDocument: {
      defaultView: {
        requestAnimationFrame(callback: FrameRequestCallback) {
          const id = ++next;
          frames.set(id, callback);
          return id;
        },
        cancelAnimationFrame(id: number) {
          frames.delete(id);
        },
        ResizeObserver: Observer,
      },
    },
    scrollTo(options: ScrollToOptions) {
      element.scrollTop = options.top ?? 0;
    },
  });
  const states: boolean[] = [];
  const follow = attachFollowLatest(element as unknown as HTMLElement, {
    thresholdPx: 20,
    reducedMotion: true,
    onFollowing: (value) => states.push(value),
  });
  const flush = () => {
    const pending = [...frames.values()];
    frames.clear();
    for (const callback of pending) callback(0);
  };
  return {
    element,
    follow,
    states,
    flush,
    frames,
    resize: () => frame?.(0),
    get observed() {
      return observed;
    },
    get disconnected() {
      return disconnected;
    },
  };
}

test("content notifications coalesce and keep the view at its latest item", () => {
  const f = fixture();
  f.element.scrollHeight = 1200;
  f.follow.notifyContent();
  f.follow.notifyContent();
  assert.equal(f.frames.size, 1);
  f.flush();
  assert.equal(f.element.scrollTop, 1000);
  assert.deepEqual(f.follow.snapshot(), { following: true });
});

test("manual upward scroll cancels following until explicit resume", () => {
  const f = fixture();
  f.element.scrollTop = 400;
  f.element.dispatchEvent(new Event("scroll"));
  assert.deepEqual(f.states, [false]);
  f.element.scrollHeight = 1200;
  f.follow.notifyContent();
  f.flush();
  assert.equal(f.element.scrollTop, 400);
  f.follow.resume();
  f.flush();
  assert.equal(f.element.scrollTop, 1000);
  assert.deepEqual(f.states, [false, true]);
});

test("a manual scroll after queued content wins over the pending frame", () => {
  const f = fixture();
  f.follow.notifyContent();
  f.element.scrollTop = 300;
  f.element.dispatchEvent(new Event("scroll"));
  f.flush();
  assert.equal(f.element.scrollTop, 300);
});

test("disposal cancels queued frames, observers and future scroll effects", () => {
  const f = fixture();
  assert.equal(f.observed, true);
  f.follow.notifyContent();
  f.follow.dispose();
  assert.equal(f.frames.size, 0);
  assert.equal(f.disconnected, true);
  f.follow.resume();
  f.follow.notifyContent();
  f.resize();
  f.element.scrollTop = 300;
  f.element.dispatchEvent(new Event("scroll"));
  f.flush();
  assert.equal(f.element.scrollTop, 300);
  assert.deepEqual(f.states, []);
});

test("invalid thresholds are rejected before installing observers", () => {
  const f = fixture();
  for (const thresholdPx of [-1, NaN, Infinity]) {
    assert.throws(
      () =>
        attachFollowLatest(f.element as unknown as HTMLElement, {
          thresholdPx,
          reducedMotion: true,
          onFollowing: () => {},
        }),
      /threshold/i,
    );
  }
  f.follow.dispose();
});

test("a viewport resize scroll event without upward movement keeps following", () => {
  const f = fixture();
  f.element.clientHeight = 100;
  f.element.dispatchEvent(new Event("scroll"));
  f.resize();
  f.flush();
  assert.equal(f.element.scrollTop, 900);
  assert.deepEqual(f.follow.snapshot(), { following: true });
});

test("even a small upward movement stops following until explicit resume", () => {
  const f = fixture();
  f.element.scrollTop = 790;
  f.element.dispatchEvent(new Event("scroll"));
  assert.deepEqual(f.follow.snapshot(), { following: false });
  f.element.scrollHeight = 1200;
  f.follow.notifyContent();
  f.flush();
  assert.equal(f.element.scrollTop, 790);
  f.element.scrollTop = 1000;
  f.element.dispatchEvent(new Event("scroll"));
  assert.deepEqual(f.follow.snapshot(), { following: false });
  f.follow.resume();
  f.flush();
  assert.deepEqual(f.follow.snapshot(), { following: true });
});

test("viewport enlargement clamping the previous position does not count as manual upward movement", () => {
  const f = fixture();
  f.element.clientHeight = 400;
  f.element.scrollTop = 600;
  f.element.dispatchEvent(new Event("scroll"));
  f.resize();
  f.flush();
  assert.deepEqual(f.follow.snapshot(), { following: true });
  assert.deepEqual(f.states, []);
});
