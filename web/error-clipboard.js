// The game dispatches clicks on a render tick; Safari may require another DOM gesture.
(() => {
  let dialog;
  let text;
  let status;
  let copy;
  let strings;
  const labels = {
    en: {
      title: 'Script error log', copy: 'Copy all', close: 'Close',
      copied: 'Copied to clipboard.',
      manual: 'Copy was blocked. Select the text and copy it manually.',
      ready: 'Copy the complete log to share it for debugging.',
    },
    fr: {
      title: "Journal d'erreurs", copy: 'Tout copier', close: 'Fermer',
      copied: 'Journal copié dans le presse-papier.',
      manual: 'Copie refusée. Sélectionnez le texte et copiez-le manuellement.',
      ready: 'Copiez le journal complet pour le partager lors du diagnostic.',
    },
  };
  async function write() {
    try {
      await navigator.clipboard.writeText(text.value);
      status.textContent = strings.copied;
      return true;
    } catch (_) {
      status.textContent = strings.manual;
      text.focus();
      text.select();
      return false;
    }
  }
  window.__wenilla_copy_errors = async (report, locale) => {
    strings = labels[locale === 'frFR' ? 'fr' : 'en'];
    if (!dialog) {
      dialog = document.createElement('dialog');
      dialog.style.cssText = 'width:min(90vw,720px);max-height:85vh;background:#0b1824;color:#d8e6ee;border:2px solid #668da4;border-radius:8px;font:16px sans-serif';
      const title = document.createElement('h2');
      title.id = 'error-export-title';
      dialog.setAttribute('aria-labelledby', title.id);
      text = document.createElement('textarea');
      text.readOnly = true;
      text.style.cssText = 'box-sizing:border-box;width:100%;height:50vh;font:13px monospace';
      status = document.createElement('p');
      status.setAttribute('role', 'status');
      copy = document.createElement('button');
      copy.onclick = write;
      const close = document.createElement('button');
      close.onclick = () => dialog.close();
      dialog.append(title, text, status, copy, close);
      document.body.append(dialog);
    }
    dialog.querySelector('h2').textContent = strings.title;
    text.setAttribute('aria-label', strings.title);
    text.value = report;
    copy.textContent = strings.copy;
    dialog.lastElementChild.textContent = strings.close;
    status.textContent = strings.ready;
    if (!dialog.open) dialog.showModal();
    // Try immediately on Chrome; the dialog's button provides a fresh gesture if refused.
    await write();
  };
})();
