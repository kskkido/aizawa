import { openAsBlob } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { createClient } from "@sanity/client";

type Metadata = {
  title: string;
  subtitle: string;
  description: string;
  images: string[];
};

type ProfileMetadata = {
  name: string;
  description: string[];
  image: {
    url: string;
    alt: string;
    caption: string;
  };
  socials: { youtube: string; instagram: string };
};

const client = createClient({
  projectId: process.env.SANITY_STUDIO_PROJECT_ID,
  dataset: process.env.SANITY_STUDIO_DATASET,
  apiVersion: "2024-01-01",
  token: process.env.SANITY_API_TOKEN,
  useCdn: false,
});

const PROJECTS_DIR = join(import.meta.dirname, "seeds", "projects");
const PROFILE_DIR = join(import.meta.dirname, "seeds", "profile");

function parseDescription(description: string): string[] {
  const lines = description.split(
    /(?=(?:Client|Creative Direction|Art Direction|Design|Project Management|Production|Project Manager|Project Lead|Design Assistance|Photography|Photograph|Japanese Calligraphy Installation|Translation|Special Thanks|Medium|Link|Edit|Editor|Editors|Managing Editor|Assistant Editor|Editor in Chief|Text|Contributors|Art Direction & Design|Edit & Text|Creative Director|Director|Cover Art|Logo Design|Produce|Producer|Skipper|Title Graphic & Design|Guide) - )/,
  );

  return lines.map((line) => line.trim()).filter(Boolean);
}

function descriptionToPortableText(description: string) {
  const lines = parseDescription(description);

  return lines.map((line) => ({
    _type: "block" as const,
    _key: crypto.randomUUID().slice(0, 8),
    style: "normal" as const,
    markDefs: [],
    children: [
      {
        _type: "span" as const,
        _key: crypto.randomUUID().slice(0, 8),
        text: line,
        marks: [],
      },
    ],
  }));
}

function getOrderedImageFiles(
  dirEntries: string[],
  metadataImages: string[],
): string[] {
  const imageExtensions = [".jpg", ".jpeg", ".png", ".gif", ".webp"];
  const localImages = dirEntries.filter((entry) => {
    const lower = entry.toLowerCase();
    return imageExtensions.some((ext) => lower.endsWith(ext));
  });

  // Map metadata URL order to local files by matching filenames
  return metadataImages
    .map((url) => {
      const urlPath = new URL(url).pathname;
      const urlFilename = urlPath.split("/").pop();
      return localImages.find(
        (local) => urlFilename && local.startsWith(urlFilename.split("_")[0]),
      );
    })
    .filter((file): file is string => file !== undefined);
}

async function uploadImage(
  filePath: string,
): Promise<{ _id: string; _type: string }> {
  const blob = await openAsBlob(filePath);
  return client.assets.upload("image", blob, {
    filename: filePath.split("/").pop(),
  });
}

async function seedProject(slug: string) {
  const projectDir = join(PROJECTS_DIR, slug);
  const metadataRaw = await readFile(
    join(projectDir, "metadata.json"),
    "utf-8",
  );
  const metadata: Metadata = JSON.parse(metadataRaw);
  const dirEntries = await readdir(projectDir);
  const imageFiles = getOrderedImageFiles(dirEntries, metadata.images);

  console.log(
    `Seeding "${metadata.title}" (${slug}) — ${imageFiles.length} images`,
  );

  const mediaIds = [];
  for (const file of imageFiles) {
    const filePath = join(projectDir, file);
    console.log(`  Uploading ${file}...`);
    const asset = await uploadImage(filePath);
    const media = await client.create({
      _type: "media",
      image: {
        _type: "image",
        asset: {
          _type: "reference",
          _ref: asset._id,
        },
      },
      alt: metadata.title,
    });
    mediaIds.push(media._id);
  }

  const document = {
    _type: "project",
    title: metadata.title,
    subtitle: metadata.subtitle,
    slug: {
      _type: "slug",
      current: slug,
    },
    content: descriptionToPortableText(metadata.description),
    rows: mediaIds.map((id) => ({
      _type: "row",
      _key: crypto.randomUUID().slice(0, 8),
      images: [
        {
          _type: "reference",
          _key: crypto.randomUUID().slice(0, 8),
          _ref: id,
        },
      ],
    })),
  };

  const result = await client.create(document);
  console.log(`  Created project ${result._id}`);
}

async function deleteProject(slug: string) {
  const ids: string[] = await client.fetch(
    `*[_type == "project" && slug.current == $slug]._id`,
    { slug },
  );
  if (ids.length === 0) return;
  for (const id of ids) {
    // Find and clear all documents referencing this project
    const referencingIds: string[] = await client.fetch(
      `*[references($id)]._id`,
      { id },
    );
    if (referencingIds.length > 0) {
      const tx = client.transaction();
      for (const refId of referencingIds) {
        // Unset slides array which contains project references
        tx.patch(refId, (p) => p.unset(["slides"]));
      }
      await tx.commit();
    }
    await client.delete(id);
  }
  console.log(`  Deleted ${ids.length} existing document(s) for "${slug}"`);
}

function linesToPortableText(lines: string[]) {
  const groups: string[][] = [];
  let current: string[] = [];

  for (const line of lines) {
    if (line === "") {
      if (current.length > 0) {
        groups.push(current);
        current = [];
      }
    } else {
      current.push(line);
    }
  }
  if (current.length > 0) {
    groups.push(current);
  }

  return groups.map((group) => ({
    _type: "block" as const,
    _key: crypto.randomUUID().slice(0, 8),
    style: "normal" as const,
    markDefs: [],
    children: group.flatMap((line, i) => {
      const span = {
        _type: "span" as const,
        _key: crypto.randomUUID().slice(0, 8),
        text: line,
        marks: [] as string[],
      };
      if (i === group.length - 1) return [span];
      return [
        span,
        { _type: "break" as const, _key: crypto.randomUUID().slice(0, 8) },
      ];
    }),
  }));
}

async function seedProfile() {
  console.log("Seeding profile...");

  const ids: string[] = await client.fetch(`*[_type == "profile"]._id`);
  if (ids.length > 0) {
    const transaction = client.transaction();
    for (const id of ids) {
      transaction.delete(id);
    }
    await transaction.commit();
    console.log(`  Deleted ${ids.length} existing profile document(s)`);
  }

  const metadataRaw = await readFile(
    join(PROFILE_DIR, "metadata.json"),
    "utf-8",
  );
  const metadata: ProfileMetadata = JSON.parse(metadataRaw);

  const filePath = join(PROFILE_DIR, metadata.image.url);
  console.log(`  Uploading ${metadata.image.url}...`);
  const imageAsset = await uploadImage(filePath);

  const media = await client.create({
    _type: "media",
    image: {
      _type: "image",
      asset: {
        _type: "reference",
        _ref: imageAsset._id,
      },
    },
    alt: metadata.image.alt,
    caption: linesToPortableText([metadata.image.caption]),
  });

  const document = {
    _id: "profile",
    _type: "profile",
    content: linesToPortableText(metadata.description),
    image: {
      _type: "reference",
      _ref: media._id,
    },
  };

  const result = await client.create(document);
  console.log(`  Created profile ${result._id}`);
}

async function deleteAllMedia() {
  const ids: string[] = await client.fetch(`*[_type == "media"]._id`);
  if (ids.length === 0) return;
  for (const id of ids) {
    const referencingIds: string[] = await client.fetch(
      `*[references($id)]._id`,
      { id },
    );
    if (referencingIds.length > 0) {
      const tx = client.transaction();
      for (const refId of referencingIds) {
        tx.patch(refId, (p) => p.unset(["image", "rows"]));
      }
      await tx.commit();
    }
    await client.delete(id);
  }
  console.log(`  Deleted ${ids.length} media document(s)`);
}

async function seedAllProjects() {
  const projects = await readdir(PROJECTS_DIR);
  for (const slug of projects) {
    await deleteProject(slug);
  }
  await deleteAllMedia();
  for (const slug of projects) {
    await seedProject(slug);
  }
  console.log(`Done. Seeded ${projects.length} projects.`);
}

async function seedHome() {
  console.log("Seeding home...");

  // Delete all existing home documents
  const ids: string[] = await client.fetch(`*[_type == "home"]._id`);
  for (const id of ids) {
    await client.delete(id);
  }
  if (ids.length > 0) {
    console.log(`  Deleted ${ids.length} existing home document(s)`);
  }

  const projects: Array<{ _id: string }> = await client.fetch(
    `*[_type == "project"]|order(_id desc)[0...6]{ _id }`,
  );

  await client.createOrReplace({
    _id: "home",
    _type: "home",
    slides: projects.map(({ _id }) => ({
      _type: "reference",
      _key: crypto.randomUUID().slice(0, 8),
      _ref: _id,
    })),
  });

  console.log(`  Created home with ${projects.length} slides`);
}

const SEEDS = {
  project: seedAllProjects,
  profile: seedProfile,
  home: seedHome,
};

const VALID_TYPES = Object.keys(SEEDS);

async function main() {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.error(`Usage: bun run seed.ts <${VALID_TYPES.join("|")}|all>`);
    process.exit(1);
  }

  const types = args.includes("all") ? VALID_TYPES : args;

  for (const type of types) {
    if (type in SEEDS) {
      await SEEDS[type as keyof typeof SEEDS]();
    } else {
      console.error(
        `Unknown type: "${type}". Valid types: ${VALID_TYPES.join(", ")}, all`,
      );
      process.exit(1);
    }
  }
}

main();
