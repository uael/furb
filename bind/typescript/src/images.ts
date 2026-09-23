import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";
import type { ImageContent } from "@earendil-works/pi-ai";
import { isTag, type Turn } from "./types.js";

export interface ImageAttachment {
  name: string;
  uri: string;
  mimeType: string;
  size: number;
}
export function imageType(data: Uint8Array): { mimeType: string; extension: string } {
  const bytes = Buffer.from(data);
  if (bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])))
    return { mimeType: "image/png", extension: "png" };
  if (bytes[0] === 255 && bytes[1] === 216 && bytes[2] === 255)
    return { mimeType: "image/jpeg", extension: "jpg" };
  if (["GIF87a", "GIF89a"].includes(bytes.subarray(0, 6).toString()))
    return { mimeType: "image/gif", extension: "gif" };
  if (bytes.subarray(0, 4).toString() === "RIFF" && bytes.subarray(8, 12).toString() === "WEBP")
    return { mimeType: "image/webp", extension: "webp" };
  throw new Error("Choose a PNG, JPEG, GIF, or WebP image.");
}
export function attachImage(directory: string, path: string): ImageAttachment {
  const info = statSync(path);
  if (!info.isFile() || info.size > 20 * 1024 * 1024)
    throw new Error("An image must be a file of at most 20 MiB.");
  const bytes = readFileSync(path);
  const { mimeType, extension } = imageType(bytes);
  const digest = createHash("sha256").update(bytes).digest("hex");
  mkdirSync(directory, { recursive: true });
  const file = `${digest}.${extension}`;
  try {
    writeFileSync(join(directory, file), bytes, { flag: "wx", mode: 0o600 });
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
  }
  return { name: basename(path), uri: `furb-image://${file}`, mimeType, size: bytes.length };
}
function imagePath(directory: string, uri: string): { path: string; digest: string } {
  const match = uri.match(/^furb-image:\/\/([a-f0-9]{64})\.(png|jpg|gif|webp)$/);
  if (!match) throw new Error("Invalid image attachment.");
  return { path: join(directory, `${match[1]}.${match[2]}`), digest: match[1] ?? "" };
}
export function imageContent(directory: string, uri: string): ImageContent {
  const { path, digest } = imagePath(directory, uri);
  if (statSync(path).size > 20 * 1024 * 1024) throw new Error("The saved image is too large.");
  const bytes = readFileSync(path);
  if (createHash("sha256").update(bytes).digest("hex") !== digest)
    throw new Error("The saved image attachment has changed.");
  return { type: "image", data: bytes.toString("base64"), mimeType: imageType(bytes).mimeType };
}
export class ImageCache {
  private readonly entries = new Map<string, { stamp: string; content: ImageContent }>();
  get(directory: string, uri: string): ImageContent {
    const { path } = imagePath(directory, uri);
    const info = statSync(path);
    if (info.size > 20 * 1024 * 1024) throw new Error("The saved image is too large.");
    const stamp = `${info.ino}:${info.size}:${info.mtimeMs}:${info.ctimeMs}`;
    let entry = this.entries.get(path);
    if (!entry || entry.stamp !== stamp) {
      entry = { stamp, content: imageContent(directory, uri) };
      this.entries.set(path, entry);
    }
    return { ...entry.content };
  }
  clear(): void {
    this.entries.clear();
  }
}
export function turnImages(directory: string, parts: Turn[1], cache?: ImageCache): ImageContent[] {
  const uris = new Set<string>();
  for (const part of parts) {
    if (!isTag(part) || part[0] !== "opened") continue;
    const message = part[1].find(([key]) => key === "message")?.[1];
    if (typeof message !== "string") continue;
    for (const match of message.matchAll(/furb-image:\/\/[a-f0-9]{64}\.(?:png|jpg|gif|webp)/g))
      uris.add(match[0]);
  }
  return [...uris].map((uri) => (cache ? cache.get(directory, uri) : imageContent(directory, uri)));
}
