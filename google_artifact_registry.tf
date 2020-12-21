resource "google_artifact_registry_repository" "images_repository" {
  provider = google-beta

  location = var.region
  repository_id = "docker-repository"
  description = "Our docker repository"
  format = "DOCKER"
}

resource "google_service_account" "gar_writer_sa" {
  account_id   = "garwriter"
  display_name = "Service Account for pushing images into the GAR"
}

resource "google_project_iam_binding" "gar_writer_binding" {
  project = var.project_id
  role    = "roles/artifactregistry.writer"

  members = [
    "serviceAccount:${google_service_account.gar_writer_sa.email}"
  ]
}
resource "google_service_account_key" "github_key" {
  service_account_id = google_service_account.gar_writer_sa.name
}

output "sa_key" {
  value = google_service_account_key.github_key.private_key
}
