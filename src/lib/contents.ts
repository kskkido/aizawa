import { defineQuery } from "groq";
import * as repo from "@/lib/repo";

const siteSettingsQuery = defineQuery(`*[_type == "site-settings"][0]`);

export function getSiteSettings() {
  return repo.read({
    query: siteSettingsQuery,
  });
}

const mediaProjection = `{
  alt,
  caption,
  "asset": image.asset->{
    ...,
    metadata { dimensions }
  }
}`;

const homeQuery = defineQuery(`*[_type == "home"]{
  slides[]->{title,subtitle,slug,"image": rows[0].images[0]->${mediaProjection}}
  }[0]`);

export function getHome() {
  return repo.read({
    query: homeQuery,
  });
}

const profileQuery = defineQuery(`*[_type == "profile"]{
    ...,
    "image": image->${mediaProjection}
  }[0]`);

export function getProfile() {
  return repo.read({
    query: profileQuery,
  });
}

const projectsQuery = defineQuery(`*[_type == "project"]|order(_id desc){
    ...,
    "image": rows[0].images[0]->${mediaProjection}
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
    "image": rows[0].images[0]->${mediaProjection},
    rows[]{
      ...,
      images[]->${mediaProjection}
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
    "image": rows[0].images[0]->${mediaProjection}
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
    "image": rows[0].images[0]->${mediaProjection}
  }[0]`);

export function getNextProject(id: string) {
  return repo.read({
    query: nextProjectQuery,
    params: { id },
  });
}
