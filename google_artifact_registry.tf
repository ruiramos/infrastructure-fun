resource "google_artifact_registry_repository" "images-repository" {
  provider = google-beta

  location = var.region
  repository_id = "docker-repository"
  description = "Our docker repository"
  format = "DOCKER"
}

