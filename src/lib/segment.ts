/**
 * pain ai — Sentence Segmentation Engine (segment.ts)
 * 
 * Splits incoming agent response text into discrete, spoken sentences for dual-render
 * TTS audio generation and real-time caption synchronization.
 * Guards against false splits on abbreviations, decimals, and quote boundaries.
 */

export interface CaptionSentenceItem {
  i: number;
  text: string;
}

const ABBREVIATIONS = new Set([
  'mr', 'mrs', 'ms', 'dr', 'prof', 'sr', 'jr', 'st', 'no', 'vs',
  'eg', 'ie', 'etc', 'am', 'pm', 'fig', 'al', 'dept', 'gen', 'gov',
  'jan', 'feb', 'mar', 'apr', 'jun', 'jul', 'aug', 'sep', 'sept', 'oct', 'nov', 'dec'
]);

const PUNCT_SPLIT_REGEX = /([.!?…]+["']?)(?:\s+|$)/g;

function isAbbreviation(prefix: string): boolean {
  const cleaned = prefix.trim().toLowerCase().replace(/\./g, '');
  if (!cleaned) return false;
  if (ABBREVIATIONS.has(cleaned)) return true;
  // Single letter initial (e.g. "J." in "J. Doe")
  if (cleaned.length === 1 && /[a-z]/i.test(cleaned)) return true;
  return false;
}

export function splitSentences(text: string): CaptionSentenceItem[] {
  if (!text || !text.trim()) {
    return [];
  }

  const rawText = text.trim();
  const placeholder = '\u2060'; // Word joiner non-printing character

  // Protect known abbreviations
  let protectedText = rawText.replace(/\b([a-zA-Z]{1,5})\./g, (match, word) => {
    if (isAbbreviation(word)) {
      return word + placeholder;
    }
    return match;
  });

  // Protect decimals: 3.14 -> 3\u206014
  protectedText = protectedText.replace(/(\d+)\.(\d+)/g, `$1${placeholder}$2`);

  // Split on sentence boundaries: [.!?…]+ followed by whitespace or end of string
  const parts: string[] = [];
  let lastIdx = 0;

  // Reset regex state
  PUNCT_SPLIT_REGEX.lastIndex = 0;

  while (PUNCT_SPLIT_REGEX.exec(protectedText) !== null) {
    const endPos = PUNCT_SPLIT_REGEX.lastIndex;
    const candidate = protectedText.substring(lastIdx, endPos).trim();
    if (candidate) {
      parts.push(candidate);
    }
    lastIdx = endPos;
  }

  if (lastIdx < protectedText.length) {
    const remainder = protectedText.substring(lastIdx).trim();
    if (remainder) {
      parts.push(remainder);
    }
  }

  // Restore placeholders and build sentence items
  const sentences: CaptionSentenceItem[] = [];
  let itemIdx = 0;

  for (const chunk of parts) {
    const restored = chunk.replaceAll(placeholder, '.').trim();
    if (restored) {
      sentences.push({
        i: itemIdx,
        text: restored,
      });
      itemIdx++;
    }
  }

  if (sentences.length === 0 && rawText) {
    sentences.push({ i: 0, text: rawText });
  }

  return sentences;
}
