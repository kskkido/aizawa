import type { ClientReturn, RawQueryResponse } from "@sanity/client";
import { createClient } from "@sanity/client";
import {
  createImageUrlBuilder,
  type SanityImageSource,
} from "@sanity/image-url";
import type { QueryParams } from "sanity";

const client = createClient({
  projectId: process.env.SANITY_STUDIO_PROJECT_ID,
  dataset: process.env.SANITY_STUDIO_DATASET,
  token: process.env.SANITY_API_TOKEN,
  apiVersion: "2024-01-01",
  perspective: "published",
});

const imageUrlBuilder = createImageUrlBuilder(client);

// only called at build time so coercing is fine. let it fail
export function read<const Query extends string>({
  query,
  params,
}: {
  query: Query;
  params?: QueryParams;
}) {
  return client.fetch(query, params ?? {}, {
    filterResponse: false,
  }) as Promise<RawQueryResponse<DeepNonNullable<ClientReturn<Query>>>>;
}

export function urlForImage(source: SanityImageSource) {
  return imageUrlBuilder.image(source);
}
