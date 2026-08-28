import { visionTool } from "@sanity/vision";
import {
  defineArrayMember,
  defineConfig,
  defineField,
  defineType,
} from "sanity";
import { structureTool } from "sanity/structure";

const singletonActions = new Set(["publish", "discardChanges", "restore"]);
const singletonTypes = new Set(["site-settings", "home", "profile"]);

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
            ...S.documentTypeListItems().filter((listItem) => {
              const id = listItem.getId();
              return id && !singletonTypes.has(id);
            }),
          ]),
    }),
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
            type: "reference",
            name: "image",
            title: "Image",
            to: [{ type: "media" }],
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
        name: "media",
        title: "Image",
        fields: [
          defineField({
            type: "image",
            name: "image",
            title: "Image",
            validation: (rule) => rule.required(),
          }),
          defineField({
            type: "string",
            name: "alt",
            title: "Alternative text",
            validation: (rule) => rule.required(),
          }),
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
        preview: {
          select: {
            title: "alt",
            media: "image",
          },
        },
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
                    name: "images",
                    title: "Images",
                    of: [
                      defineArrayMember({
                        type: "reference",
                        to: [{ type: "media" }],
                      }),
                    ],
                    validation: (rule) => rule.required().min(1).max(3),
                  }),
                ],
                preview: {
                  select: {
                    images: "images",
                  },
                  prepare({ images }) {
                    const count = images?.length ?? 0;
                    return {
                      title: `Row — ${count} image${count === 1 ? "" : "s"}`,
                    };
                  },
                },
              }),
            ],
            validation: (rule) => rule.required(),
          }),
        ],
      }),
    ],
    templates: (templates) =>
      templates.filter(({ schemaType }) => !singletonTypes.has(schemaType)),
  },
  document: {
    actions: (input, context) =>
      singletonTypes.has(context.schemaType)
        ? input.filter(({ action }) => action && singletonActions.has(action))
        : input,
  },
});
