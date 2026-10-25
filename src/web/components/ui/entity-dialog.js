// @ts-check

/**
 * Entity details. Clicking a `[data-entity="kind:id"]` element opens a modal
 * that shows "Loading…", then the HTML of `/entity/kind/id` as is.
 */
class EntityDialog {
  /** @type {HTMLDialogElement} */
  #dialog;
  /** @type {HTMLDivElement} */
  #body;
  /** Cancels the pending request when the dialog closes or reopens. */
  /** @type {AbortController | null} */
  #request = null;

  constructor() {
    this.#dialog = document.createElement('dialog');
    this.#dialog.className = 'modal';

    const close = document.createElement('button');
    close.className = 'modal-close';
    close.type = 'button';
    close.ariaLabel = 'Close';
    close.textContent = '×';
    close.addEventListener('click', () => this.#dialog.close());

    this.#body = document.createElement('div');
    this.#body.className = 'modal-body';

    this.#dialog.append(close, this.#body);
    document.body.append(this.#dialog);

    // A click on the backdrop lands on the dialog itself.
    this.#dialog.addEventListener('click', (event) => {
      if (event.target === this.#dialog) this.#dialog.close();
    });
    // Esc closes it natively; this covers every way of closing.
    this.#dialog.addEventListener('close', () => {
      this.#request?.abort();
      this.#request = null;
    });
    document.addEventListener('click', (event) => this.#click(event));
  }

  /** @param {MouseEvent} event */
  #click(event) {
    if (!(event.target instanceof Element)) return;
    const target = event.target.closest('[data-entity]');
    if (!(target instanceof HTMLElement)) return;
    const entity = target.dataset.entity ?? '';
    const colon = entity.indexOf(':');
    if (colon < 0) return;
    this.open(entity.slice(0, colon), entity.slice(colon + 1));
  }

  /**
   * @param {string} kind e.g. "app"
   * @param {string} id
   */
  async open(kind, id) {
    this.#request?.abort();
    const request = new AbortController();
    this.#request = request;

    this.#body.textContent = 'Loading…';
    if (!this.#dialog.open) this.#dialog.showModal();

    const url = `/entity/${encodeURIComponent(kind)}/${encodeURIComponent(id)}`;
    try {
      const response = await fetch(url, { signal: request.signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const html = await response.text();
      if (!request.signal.aborted) this.#body.innerHTML = html;
    } catch (error) {
      if (request.signal.aborted) return;
      const reason = error instanceof Error ? error.message : String(error);
      this.#body.textContent = `Failed to load: ${reason}`;
    }
  }
}

new EntityDialog();
