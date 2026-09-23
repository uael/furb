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
/** The uri of an image attachment: the digest of its bytes and the extension of its type. */
const uriForm = String.raw`furb-image://(?<digest>[a-f0-9]{64})\.(?<extension>png|jpg|gif|webp)`;
/** The file that holds an image attachment in a directory of images, and the digest its bytes must have. */
export function imagePath(directory: string, reference: string): { path: string; digest: string } {
  const { digest, extension } = reference.match(new RegExp(`^${uriForm}$`))?.groups ?? {};
  if (!digest || !extension) throw new Error("Invalid image attachment.");
  return { path: join(directory, `${digest}.${extension}`), digest };
}
/** An image attachment as a message holds it: `![name](uri)`. */
export function imageReference(image: { name: string; uri: string }): string {
  return `![${image.name.replace(/[[\]\r\n]/g, "_")}](${image.uri})`;
}
/** The image attachments that a message holds, each with its text in the message, its name, and its uri. */
export function imageReferences(message: string): { text: string; name: string; uri: string }[] {
  return [...message.matchAll(new RegExp(String.raw`!\[(?<name>[^\]]*)\]\((?<uri>${uriForm})\)`, "g"))].map(
    (match) => ({ text: match[0], name: match.groups?.name ?? "", uri: match.groups?.uri ?? "" }),
  );
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
    for (const reference of imageReferences(message)) uris.add(reference.uri);
  }
  return [...uris].map((uri) => (cache ? cache.get(directory, uri) : imageContent(directory, uri)));
}
