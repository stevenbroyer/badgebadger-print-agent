// Sorting a computer's printer list into what an office actually has: card
// printers (what badges go to), ordinary printers, and the virtual ones
// Windows and apps add (PDF, OneNote, fax, remote-support drivers) that
// nobody prints a badge on.

// Card printer families by the names their drivers install under. Zebra
// only counts for its card lines (ZC, ZXP); its ZD/GK/ZT printers are label
// printers.
const CARD_PRINTER =
  /\b(card printer|id card|zebra\s+(zc|zxp)|zc[1-3]\d0|zxp|evolis|primacy|zenius|avansia|badgy\d*|fargo|dtc\d{3,4}|hdp\d{4}|magicard|enduro|rio pro|pronto|datacard|entrust|sigma ds|cd8\d0|sd[2-4]60|idp smart|smart-?[357]\d|matica|nisca|edisecure|swiftcolor)\b/i;

const VIRTUAL_PRINTER =
  /\b(pdf|onenote|xps|fax|logmein|send to|snagit|anydesk|teamviewer|splashtop|cutepdf|bullzip|nitro|print to|document writer|webex|zoom)\b/i;

export type PrinterKind = "card" | "office" | "virtual";

export function printerKind(name: string): PrinterKind {
  if (CARD_PRINTER.test(name)) return "card";
  if (VIRTUAL_PRINTER.test(name)) return "virtual";
  return "office";
}

const ORDER: Record<PrinterKind, number> = { card: 0, office: 1, virtual: 2 };

/** Card printers first, virtual printers last, A to Z within each. */
export function sortPrinters(printers: string[]): string[] {
  return [...printers].sort(
    (a, b) =>
      ORDER[printerKind(a)] - ORDER[printerKind(b)] ||
      a.localeCompare(b, undefined, { sensitivity: "base" }),
  );
}
