# How to edit website content

All of the website's text, images and videos are managed in **Sanity Studio**, an online editor. This guide explains how to add, change and remove content.

## URLs
- [Sanity Studio](https://www.sanity.io/@odz52Iitd/studio/rgi0143or3neun5rokrgqrlu/default/structure)
- [Your website](https://aizawa.kskkido.workers.dev/)

## Contents

- [Before you start](#before-you-start)
- [Saving and publishing](#saving-and-publishing)
- [Site Settings](#site-settings)
- [Profile (Info page)](#profile-info-page)
- [Projects](#projects)
- [Home slideshow](#home-slideshow)
- [Images, videos and alt text](#images-videos-and-alt-text)
- [Writing text](#writing-text)
- [Quick reference](#quick-reference)

## Before you start

Open [Sanity Studio](https://aizawa.kskkido.workers.dev/) and sign in. The left-hand menu has these sections:

| Section | What it controls |
|---|---|
| **Site Settings** | The site name and description shown in browser tabs, search results and link previews |
| **Home** | Which projects appear in the home page slideshow |
| **Profile** | The Info page: your biography and portrait |
| **Projects** | Which projects appear on the Projects page, and in what order |
| **Project** | The individual projects themselves |

Site Settings, Home, Profile and Projects each have exactly one page. You can edit them, but you can't create new ones or delete them. **Project** holds all of your projects, and you can add and remove those freely.

## Saving and publishing

- **Changes save automatically** as a draft while you type. There is no Save button.
- **Drafts are private.** Nothing you change is visible on the website until you press **Publish** (bottom right).
- **Publishing doesn't update the website instantly.** The website is rebuilt from the published content, so a change appears on the live site after the next rebuild, not the moment you press Publish.
- **To throw away unpublished changes**, open the menu next to Publish and choose **Discard changes**. The page goes back to its last published version.
- **To go back to an older version**, open the history (clock icon at the top of the page), choose a past version, and press **Restore**. Then publish.

If a page has a problem, such as a required field left empty, the Publish button is disabled and the problem is highlighted in red. Fix it and the button becomes available again.

## Site Settings

| Field | Where it appears |
|---|---|
| **Title** | The site name in browser tabs, and when the site is shared on social media. Required. |
| **Description** | The short summary shown in search results and link previews. Required. |

## Profile (Info page)

| Field | Where it appears |
|---|---|
| **Description** | Your biography on the Info page. See [Writing text](#writing-text). |
| **Image** | Your portrait on the Info page. It's also the preview image when someone shares a page that has no project image, such as the home page. |
| **Image → Caption** | Optional text shown under the portrait. It can contain links. |

To change the portrait, click the image, choose **Replace**, and upload or select a new one.

## Projects

### Create a project

1. Click **Project** in the left-hand menu, then the **+** (Create) button.
2. Fill in the fields:

   | Field | Notes |
   |---|---|
   | **Title** | Required. |
   | **Subtitle** | Required. A short label shown under the title, for example "Editorial". |
   | **Slug** | The project's web address, for example `popeye-2024-dec` gives `/projects/popeye-2024-dec`. Click **Generate** to create it from the title. |
   | **Content** | The project description and credits. See [Writing text](#writing-text). |
   | **Rows** | The images and videos. See [Rows: images and videos](#rows-images-and-videos). Required. |

3. Press **Publish**.
4. **Add the project to the Projects list.** This step is required. A published project only gets its own page on the website if it's in this list. Open **Projects** in the left-hand menu, click **Add item**, choose your new project, and press **Publish**.
5. Optionally, add it to the [Home slideshow](#home-slideshow).

### Rows: images and videos

A project's images and videos are arranged in **rows**, from top to bottom. Each row holds **1 to 4 items**, shown side by side at equal height. Use one item per row for a full-width image, or two to four for a grid.

- **Add a row:** under Rows, click **Add item**, then add images or videos to the row's **Media**.
- **Add an item to a row:** open the row and click **Add item** under Media. Choose **Image** or **Video**.
- **Reorder:** drag rows, or items inside a row, by the handle on their left.
- **Remove:** open the menu (**⋯**) on a row or item and choose **Remove**.

**The first item of the first row is the project's cover.** It's used on the Projects page, in the home slideshow, in the "next/previous project" links, and as the preview image when the project is shared. **It must be an image, not a video.**

### Videos

When you add a Video item:

- Upload the video as an **MP4** file. That's the format all browsers play.
- Add a **Poster** image. It's shown before the video plays, and the site uses it to reserve the right amount of space for the video. Usually a still frame from the video works best.

### Edit a project

Open it from **Project**, make your changes, and press **Publish**.

**Avoid changing the Slug of a published project.** It changes the project's web address, so any links people have shared to the old address will stop working.

### Change the order of projects

The order on the Projects page is the order of the list in **Projects**. Drag items to reorder them, then press **Publish**.

The "next" and "previous" project links at the bottom of each project page are picked automatically. You can't set them by hand.

### Hide or delete a project

A project that is still in the Projects list or the Home slideshow can't be unpublished or deleted. Sanity will refuse, because other pages point to it. So first:

1. Remove the project from **Projects** (open the item's **⋯** menu, choose **Remove**) and press **Publish**.
2. If it's in the slideshow, remove it from **Home** the same way and press **Publish**.

Then open the project and choose one of these from the menu next to Publish:

- **Unpublish**: takes it off the website but keeps it in the Studio as a draft, so you can publish it again later.
- **Delete**: removes it permanently. You can't undo this.

## Home slideshow

Open **Home**. **Slides** is a list of projects shown in the home page slideshow, in order.

- **Add a slide:** click **Add item** and choose a project.
- **Reorder:** drag the items.
- **Remove:** open the item's **⋯** menu and choose **Remove**.

Press **Publish** when you're done. Each slide shows the project's cover (see [Rows: images and videos](#rows-images-and-videos)), title and subtitle.

## Images, videos and alt text

All uploaded files are kept in the **Media** library (the **Media** tab at the top of the Studio). Uploading the same file twice doesn't create a duplicate, and you can reuse a file across projects by choosing **Select** instead of **Upload** when you add an image.

**Alt text** describes an image for people who use screen readers, and for search engines. The site reads it from the Media library, not from the project:

1. Open the **Media** tab and click the image.
2. Fill in **Alt text**, for example "Magazine spread with an illustrated map of Tokyo".
3. Save.

Alt text belongs to the file, so it applies everywhere that image is used.

**Image tips:**

- Upload the largest, best-quality version you have. The website resizes images automatically for each screen.
- JPG works best for photos. Use PNG only for graphics that need sharp edges or transparency.

## Writing text

The **Content** field on projects and the **Description** field on the Profile are rich-text editors. They work like a simple word processor:

- **Paragraphs:** press **Enter**.
- **Line break** (a new line without starting a new paragraph, for example in a list of credits): click the **Insert** button in the toolbar and choose **Line Break**. It appears as **↵**. Don't use Shift+Enter: those line breaks won't show on the website.
- **Headings and quotes:** choose a style (Normal, H1–H4, Quote) from the style menu at the left of the toolbar.
- **Bulleted lists:** use the bullet list button.
- **Bold and italic:** select text and use the **B** and *I* buttons.
- **Links:** select the text, click the link button, and paste the full address including `https://`. Links to other websites open in a new tab.

## Quick reference

| I want to… | Do this |
|---|---|
| Add a new project | Create it under **Project**, publish it, then add it to **Projects** and publish that |
| Change project order | Drag items in **Projects**, then publish |
| Change the home slideshow | Edit **Slides** in **Home**, then publish |
| Change a project's cover | Move the image you want to the first item of the first row |
| Remove a project | Remove it from **Projects** and **Home**, publish both, then Unpublish or Delete the project |
| Undo unpublished edits | **Discard changes** from the menu next to Publish |
| Bring back an older version | History (clock icon), choose a version, **Restore**, then publish |
| Add alt text | **Media** tab, click the image, fill in **Alt text** |
