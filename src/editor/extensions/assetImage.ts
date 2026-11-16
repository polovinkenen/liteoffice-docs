import Image from "@tiptap/extension-image";
import type { Node as ProseMirrorNode } from "@tiptap/pm/model";
import { loadAsset } from "../../features/file/fileApi";

const ASSET_PREFIX = "asset:";
const assetCache = new Map<string, string>();
const pendingAssets = new Map<string, Promise<string>>();

function loadCachedAsset(assetId: string): Promise<string> {
  const cached = assetCache.get(assetId);

  if (cached) {
    return Promise.resolve(cached);
  }

  const pending = pendingAssets.get(assetId);

  if (pending) {
    return pending;
  }

  const request = loadAsset(assetId)
    .then((dataUrl) => {
      assetCache.set(assetId, dataUrl);
      pendingAssets.delete(assetId);
      return dataUrl;
    })
    .catch((cause) => {
      pendingAssets.delete(assetId);
      throw cause;
    });

  pendingAssets.set(assetId, request);

  return request;
}

function attribute(node: ProseMirrorNode, name: string): string {
  const value = node.attrs[name];
  return typeof value === "string" ? value : "";
}

export const AssetImage = Image.extend({
  addNodeView() {
    return ({ node }) => {
      const image = document.createElement("img");
      let disposed = false;
      let requestId = 0;

      const render = (current: ProseMirrorNode) => {
        const source = attribute(current, "src");
        const alt = attribute(current, "alt");
        const title = attribute(current, "title");
        const request = ++requestId;

        image.alt = alt;

        if (title) {
          image.title = title;
        } else {
          image.removeAttribute("title");
        }

        if (source.startsWith(ASSET_PREFIX)) {
          image.removeAttribute("src");

          void loadCachedAsset(source.slice(ASSET_PREFIX.length))
            .then((dataUrl) => {
              if (!disposed && request === requestId) {
                image.src = dataUrl;
              }
            })
            .catch(() => {
              if (!disposed && request === requestId) {
                image.removeAttribute("src");
              }
            });
        } else if (source) {
          image.src = source;
        } else {
          image.removeAttribute("src");
        }
      };

      render(node);

      return {
        dom: image,
        update: (updated: ProseMirrorNode) => {
          if (updated.type.name !== node.type.name) {
            return false;
          }

          render(updated);
          return true;
        },
        destroy: () => {
          disposed = true;
        },
      };
    };
  },
});
