/** The part of gifenc that the animation uses, since gifenc gives no types of its own. */
declare module "gifenc" {
  export function GIFEncoder(): {
    writeFrame(
      index: Uint8Array,
      width: number,
      height: number,
      options: {
        palette?: number[][];
        delay?: number;
        transparent?: boolean;
        transparentIndex?: number;
        dispose?: number;
      },
    ): void;
    finish(): void;
    bytes(): Uint8Array;
  };
}
