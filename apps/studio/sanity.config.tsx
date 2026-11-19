import { visionTool } from "@sanity/vision";
import {
  defineArrayMember,
  defineConfig,
  defineField,
  defineType,
} from "sanity";
import { structureTool } from "sanity/structure";
import { media } from "sanity-plugin-media";

const singletonActions = new Set(["publish", "discardChanges", "restore"]);
const singletonTypes = new Set([
  "site-settings",
  "home",
  "profile",
  "projects",
]);
// sanity-plugin-media stores its tags and folders as documents; manage them in the Media tool
const mediaPluginTypes = new Set(["media.tag", "media.folder"]);

export default defineConfig({
  name: "default",
  title: "aizawa",
  projectId: process.env.SANITY_STUDIO_PROJECT_ID!,
  dataset: process.env.SANITY_STUDIO_DATASET!,
  plugins: [
    structureTool({
      structure: (S) =>
        S.list()
          .title("Content")
          .items([
            S.listItem()
              .title("Site Settings")
              .id("site-settings")
              .child(
                S.document()
                  .schemaType("site-settings")
                  .documentId("site-settings"),
              ),
            S.listItem()
              .title("Home")
              .id("home")
              .child(S.document().schemaType("home").documentId("home")),
            S.listItem()
              .title("Profile")
              .id("profile")
              .child(S.document().schemaType("profile").documentId("profile")),
            S.listItem()
              .title("Projects")
              .id("projects")
              .child(
                S.document().schemaType("projects").documentId("projects"),
              ),
            ...S.documentTypeListItems().filter((listItem) => {
              const id = listItem.getId();
              return id && !singletonTypes.has(id) && !mediaPluginTypes.has(id);
            }),
          ]),
    }),
    media(),
    visionTool(),
  ],
  schema: {
    types: [
      defineType({
        type: "document",
        name: "site-settings",
        title: "Site Settings",
        fields: [
          defineField({
            type: "string",
            name: "title",
            title: "Title",
            validation: (rule) => rule.required(),
          }),
          defineField({
            type: "string",
            name: "description",
            title: "Description",
            validation: (rule) => rule.required(),
          }),
        ],
      }),
      defineType({
        type: "document",
        name: "profile",
        title: "Profile",
        fields: [
          defineField({
            type: "array",
            name: "description",
            title: "Description",
            of: [
              defineArrayMember({
                type: "block",
                styles: [
                  { title: "Normal", value: "normal" },
                  { title: "H1", value: "h1" },
                  { title: "H2", value: "h2" },
                  { title: "H3", value: "h3" },
                  { title: "H4", value: "h4" },
                  { title: "Quote", value: "blockquote" },
                ],
                lists: [{ title: "Bullet", value: "bullet" }],
                marks: {
                  decorators: [
                    { title: "Strong", value: "strong" },
                    { title: "Emphasis", value: "em" },
                  ],
                  annotations: [
                    {
                      title: "URL",
                      name: "link",
                      type: "object",
                      fields: [
                        {
                          title: "URL",
                          name: "href",
                          type: "url",
                        },
                      ],
                    },
                  ],
                },
                of: [
                  defineArrayMember({
                    name: "break",
                    title: "Line Break",
                    type: "object",
                    fields: [
                      defineField({
                        name: "style",
                        type: "string",
                        initialValue: "lineBreak",
                        hidden: true,
                      }),
                    ],
                    preview: {
                      prepare: () => ({ title: "↵" }),
                    },
                  }),
                ],
              }),
            ],
          }),
          defineField({
            type: "image",
            name: "image",
            title: "Image",
            fields: [
              defineField({
                type: "array",
                name: "caption",
                title: "Caption",
                of: [
                  defineArrayMember({
                    type: "block",
                    styles: [{ title: "Normal", value: "normal" }],
                    marks: {
                      annotations: [
                        {
                          title: "URL",
                          name: "link",
                          type: "object",
                          fields: [
                            {
                              title: "URL",
                              name: "href",
                              type: "url",
                            },
                          ],
                        },
                      ],
                    },
                  }),
                ],
              }),
            ],
          }),
        ],
      }),
      defineType({
        type: "document",
        name: "home",
        title: "Home",
        fields: [
          defineField({
            type: "array",
            name: "slides",
            title: "Slides",
            of: [
              {
                type: "reference",
                to: [{ type: "project" }],
              },
            ],
          }),
        ],
      }),
      defineType({
        type: "document",
        name: "projects",
        title: "Projects",
        fields: [
          defineField({
            type: "array",
            name: "items",
            title: "Projects",
            of: [
              defineArrayMember({
                type: "reference",
                to: [{ type: "project" }],
              }),
            ],
            options: {
              layout: "grid",
            },
          }),
        ],
      }),
      defineType({
        type: "document",
        name: "project",
        title: "Project",
        fields: [
          defineField({
            type: "string",
            name: "title",
            title: "Title",
            validation: (rule) => rule.required(),
          }),
          defineField({
            type: "string",
            name: "subtitle",
            title: "Subtitle",
            validation: (rule) => rule.required(),
          }),
          defineField({
            type: "slug",
            name: "slug",
            options: {
              source: "title",
              maxLength: 96,
            },
          }),
          defineField({
            type: "array",
            name: "content",
            title: "Content",
            of: [
              defineArrayMember({
                type: "block",
                styles: [
                  { title: "Normal", value: "normal" },
                  { title: "H1", value: "h1" },
                  { title: "H2", value: "h2" },
                  { title: "H3", value: "h3" },
                  { title: "H4", value: "h4" },
                  { title: "Quote", value: "blockquote" },
                ],
                lists: [{ title: "Bullet", value: "bullet" }],
                marks: {
                  decorators: [
                    { title: "Strong", value: "strong" },
                    { title: "Emphasis", value: "em" },
                  ],
                  annotations: [
                    {
                      title: "URL",
                      name: "link",
                      type: "object",
                      fields: [
                        {
                          title: "URL",
                          name: "href",
                          type: "url",
                        },
                      ],
                    },
                  ],
                },
                of: [
                  defineArrayMember({
                    name: "break",
                    title: "Line Break",
                    type: "object",
                    fields: [
                      defineField({
                        name: "style",
                        type: "string",
                        initialValue: "lineBreak",
                        hidden: true,
                      }),
                    ],
                    preview: {
                      prepare: () => ({ title: "↵" }),
                    },
                  }),
                ],
              }),
            ],
          }),
          defineField({
            type: "array",
            name: "rows",
            title: "Rows",
            of: [
              defineArrayMember({
                type: "object",
                name: "row",
                title: "Row",
                fields: [
                  defineField({
                    type: "array",
                    name: "media",
                    title: "Media",
                    of: [
                      defineArrayMember({
                        type: "image",
                        name: "image",
                        title: "Image",
                      }),
                      defineArrayMember({
                        type: "file",
                        name: "video",
                        title: "Video",
                        options: {
                          accept: "video/*",
                        },
                        fields: [
                          defineField({
                            type: "image",
                            name: "poster",
                            title: "Poster",
                          }),
                        ],
                      }),
                    ],
                    validation: (rule) => rule.required().min(1).max(4),
                  }),
                ],
                preview: {
                  select: {
                    media: "media",
                  },
                  prepare({ media }) {
                    const count = media?.length ?? 0;
                    return {
                      title: `Row — ${count} item${count === 1 ? "" : "s"}`,
                    };
                  },
                },
              }),
            ],
            validation: (rule) => rule.required(),
          }),
        ],
        preview: {
          select: {
            title: "title",
            subtitle: "subtitle",
            media: "rows.0.media.0",
          },
        },
      }),
    ],
    templates: (templates) =>
      templates.filter(
        ({ schemaType }) =>
          !singletonTypes.has(schemaType) && !mediaPluginTypes.has(schemaType),
      ),
  },
  document: {
    actions: (input, context) =>
      singletonTypes.has(context.schemaType)
        ? input.filter(({ action }) => action && singletonActions.has(action))
        : input,
  },
});
