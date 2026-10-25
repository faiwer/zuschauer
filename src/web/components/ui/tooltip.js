// @ts-check

/**
 * Hover hints. A `[data-hint="id"]` element shows a copy of `#id`'s HTML
 * after a delay, fixed beside it and kept inside the viewport. Touch input
 * gets none.
 */
class Tooltip {
  /** @type {HTMLDivElement} */
  #el;
  /** The hovered `[data-hint]` element, shown or pending. */
  /** @type {HTMLElement | SVGElement | null} */
  #target = null;
  /** @type {number | undefined} */
  #timer;
  /** When a tooltip last hid. A target hovered soon after skips the delay. */
  #hiddenAt = -Infinity;
  #delay;
  #warm;
  #gap;

  /**
   * @param {object} [options]
   * @param {number} [options.delay] ms of hovering before it shows
   * @param {number} [options.warm] ms after hiding in which the next shows at once
   * @param {number} [options.gap] px between the target, the tooltip and the viewport's edges
   */
  constructor({ delay = 500, warm = 300, gap = 6 } = {}) {
    this.#delay = delay;
    this.#warm = warm;
    this.#gap = gap;

    this.#el = document.createElement('div');
    this.#el.className = 'tooltip';
    this.#el.setAttribute('role', 'tooltip');
    this.#el.hidden = true;
    document.body.append(this.#el);

    document.addEventListener('pointerover', (event) => this.#over(event));
    document.addEventListener('pointerout', (event) => this.#out(event));
    const hide = () => this.hide();
    // Capture: scrolls of inner elements don't bubble.
    window.addEventListener('scroll', hide, { capture: true, passive: true });
    window.addEventListener('resize', hide);
    window.addEventListener('blur', hide);
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape') hide();
    });
  }

  hide() {
    clearTimeout(this.#timer);
    this.#target = null;
    if (!this.#el.hidden) {
      this.#el.hidden = true;
      this.#hiddenAt = performance.now();
    }
  }

  /** @param {PointerEvent} event */
  #over(event) {
    if (event.pointerType === 'touch' || !(event.target instanceof Element))
      return;
    const target = event.target.closest('[data-hint]');
    if (
      !(target instanceof HTMLElement || target instanceof SVGElement) ||
      target === this.#target
    )
      return;
    this.hide();
    this.#target = target;
    const warm = performance.now() - this.#hiddenAt < this.#warm;
    this.#timer = setTimeout(() => this.#show(target), warm ? 0 : this.#delay);
  }

  /** @param {PointerEvent} event */
  #out(event) {
    if (!this.#target) return;
    // Moving between the target's own children isn't leaving it.
    if (
      event.relatedTarget instanceof Node &&
      this.#target.contains(event.relatedTarget)
    )
      return;
    this.hide();
  }

  /** @param {HTMLElement | SVGElement} target */
  #show(target) {
    const source = document.getElementById(target.dataset.hint ?? '');
    if (!source) return;
    const el = this.#el;
    el.innerHTML = source.innerHTML;
    // Measured at the corner, so the right edge doesn't squeeze it.
    el.style.left = '0';
    el.style.top = '0';
    el.style.visibility = 'hidden';
    el.hidden = false;
    this.#place(target.getBoundingClientRect(), el.getBoundingClientRect());
    el.style.visibility = '';
  }

  /**
   * Above the target, else below; centered on it, clamped to the viewport.
   * @param {DOMRect} target
   * @param {DOMRect} tooltip
   */
  #place(target, tooltip) {
    const gap = this.#gap;
    const width = document.documentElement.clientWidth;
    const height = document.documentElement.clientHeight;

    let top = target.top - tooltip.height - gap;
    if (top < gap) {
      top = target.bottom + gap;
    }
    top = clamp(top, gap, height - gap - tooltip.height);

    const center = target.left + target.width / 2;
    const left = clamp(
      center - tooltip.width / 2,
      gap,
      width - gap - tooltip.width,
    );

    this.#el.style.left = `${left}px`;
    this.#el.style.top = `${top}px`;
  }
}

/**
 * `min` wins when the range is empty: a tooltip larger than the viewport
 * sticks to its top left.
 * @param {number} value
 * @param {number} min
 * @param {number} max
 */
function clamp(value, min, max) {
  return Math.max(min, Math.min(value, max));
}

new Tooltip();
