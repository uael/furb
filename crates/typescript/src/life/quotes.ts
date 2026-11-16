/** A quote of a word, as the crate reads it: the line where it opens, its name, its text, the line of its close mark
 * or the last line that came, whether its close mark came, and the binding that it makes. Its mark is <s:name> at the
 * start of a line. */
export interface Quote {
  start: number;
  name: string;
  body: string;
  end: number;
  closed: boolean;
  binding: string;
}

/** What a model writes while its word has not come yet, as the crate reads it. */
export interface Writing {
  lines: string;
  tail: string;
  numbers: number[];
  count: number;
}

/** The reader of the quotes of a word, which only the crate holds: a host gives it once, as the package of furb does
 * with the door of the crate, and a window of the web with the one that it reaches. */
export interface QuoteReader {
  quotes(text: string, open?: boolean): Quote[];
  writing(text: string, thinking: string): Writing;
}

let reader: QuoteReader | undefined;

/** The reader of quotes that the model of a life reads each word with. */
export function readQuotesWith(given: QuoteReader): void {
  reader = given;
}

/** The reader of quotes that a host gave. */
export function quoteReader(): QuoteReader {
  if (!reader)
    throw new Error("No reader of quotes: a host gives the one of the crate through readQuotesWith.");
  return reader;
}
