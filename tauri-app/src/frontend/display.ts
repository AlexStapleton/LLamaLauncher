const sizes = new Set(['90', '100', '112.5', '125', '150', '200']);

export function initializeTextSize(select: HTMLSelectElement) {
  let saved: string | null = null;
  try { saved = localStorage.getItem('llama_text_size'); } catch { /* Defaults remain usable without browser storage. */ }
  const apply = (value: string) => {
    const size = sizes.has(value) ? value : '100';
    document.documentElement.style.fontSize = `${size}%`;
    document.documentElement.dataset.largeText = String(Number(size) >= 150);
    select.value = size;
    try { localStorage.setItem('llama_text_size', size); } catch { /* Session preference still applies. */ }
  };
  apply(saved || '100');
  select.addEventListener('change', () => apply(select.value));
}
