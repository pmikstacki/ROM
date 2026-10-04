/** Relocate one mounted editor between stable hosts without resetting its draft. */
export function preserveEditor(
  node: HTMLDivElement,
  target: HTMLDivElement | null,
) {
  const origin = node.parentNode;
  function move(next: HTMLDivElement | null) {
    if (next && node.parentNode !== next) next.appendChild(node);
  }
  move(target);
  return {
    update: move,
    destroy() {
      if (origin && node.parentNode !== origin) origin.appendChild(node);
    },
  };
}
