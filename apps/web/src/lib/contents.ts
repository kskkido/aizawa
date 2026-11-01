import { defineQuery } from "groq";
import * as repo from "@/lib/repo";

const siteSettingsQuery = defineQuery(`*[_type == "site-settings"][0]`);

export function getSiteSettings() {
  return repo.read({
    query: siteSettingsQuery,
  });
}

const imageProjection = `{
  ...,
  "alt": asset->altText,
  asset->{
    ...,
    metadata { dimensions }
  }
}`;

const videoProjection = `{
  ...,
  "alt": asset->altText,
  "url": asset->url,
  "mimeType": asset->mimeType,
  poster{
    ...,
    "alt": asset->altText,
    asset->{
      ...,
      metadata { dimensions }
    }
  }
}`;

const mediaProjection = `{
  ...,
  _type == "image" => ${imageProjection},
  _type == "video" => ${videoProjection}
}`;

const homeQuery = defineQuery(`*[_type == "home"]{
  slides[]->{title,subtitle,slug,"image": rows[0].media[0]${imageProjection}}
  }[0]`);

export function getHome() {
  return repo.read({
    query: homeQuery,
  });
}

const profileQuery = defineQuery(`*[_type == "profile"]{
    ...,
    "image": image${imageProjection}
  }[0]`);

export function getProfile() {
  return repo.read({
    query: profileQuery,
  });
}

const projectsQuery = defineQuery(`*[_type == "projects"][0]{
    "items": items[]->{
      ...,
      "image": rows[0].media[0]${imageProjection}
    }
  }`);

export function getProjects() {
  return repo.read({
    query: projectsQuery,
  });
}

const projectQuery =
  defineQuery(`*[_type == "project" && slug.current == $slug]{
    ...,
    _id,
    "image": rows[0].media[0]${imageProjection},
    rows[]{
      ...,
      media[]${mediaProjection}
    }
  }[0]`);

export function getProject(slug: string) {
  return repo.read({
    query: projectQuery,
    params: { slug },
  });
}

const previousProjectQuery =
  defineQuery(`*[_type == "project" && _id < $id]|order(_id desc){
    ...,
    "image": rows[0].media[0]${imageProjection}
  }[0]`);

export function getPreviousProject(id: string) {
  return repo.read({
    query: previousProjectQuery,
    params: { id },
  });
}

const nextProjectQuery =
  defineQuery(`*[_type == "project" && _id > $id]|order(_id asc){
    ...,
    "image": rows[0].media[0]${imageProjection}
  }[0]`);

export function getNextProject(id: string) {
  return repo.read({
    query: nextProjectQuery,
    params: { id },
  });
}
