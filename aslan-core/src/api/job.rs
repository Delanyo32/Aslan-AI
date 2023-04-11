use actix_web::{web::{self, Json}, Responder, HttpResponse};
use apalis::{postgres::PostgresStorage, prelude::{JobState, Storage, JobStreamExt}};

use crate::types::app_state::{TrainJob, JobList};

pub async fn list_jobs(storage: web::Data<PostgresStorage<TrainJob>>) -> Json<JobList> {
    let storage = &*storage.into_inner();
    let mut storage = storage.clone();

    let mut job_state = JobState::Done;
    let done = storage.list_jobs(&job_state, 1).await.unwrap();
    let done_list = done.iter().map(|job| job.id().clone()).collect::<Vec<String>>();

    job_state = JobState::Pending;
    let pending = storage.list_jobs(&job_state, 1).await.unwrap();
    let pending_list = pending.iter().map(|job| job.id()).collect::<Vec<String>>();

    job_state = JobState::Running;
    let running = storage.list_jobs(&job_state, 1).await.unwrap();
    let running_list = running.iter().map(|job| job.id()).collect::<Vec<String>>();

    job_state = JobState::Failed;
    let failed = storage.list_jobs(&job_state, 1).await.unwrap();
    let failed_list = failed.iter().map(|job| job.id()).collect::<Vec<String>>();

    job_state = JobState::Retry;
    let retry = storage.list_jobs(&job_state, 1).await.unwrap();
    let retry_list = retry.iter().map(|job| job.id()).collect::<Vec<String>>();

    // let workers = storage.list_workers().await.unwrap();
    // let worker_list = workers.iter().map(|worker| worker.id().clone()).collect::<Vec<String>>();

    let job_list = JobList {
        done: done_list,
        pending: pending_list,
        running: running_list,
        failed: failed_list,
        retry: retry_list,
    };
    return Json(job_list);
}

pub async fn kill_job(job_id: web::Path<String>,storage: web::Data<PostgresStorage<TrainJob>>)->impl Responder {
    let storage = &*storage.into_inner();
    let storage = storage.clone();
    let mut job_details  =  storage.fetch_by_id(job_id.clone()).await.unwrap().unwrap();
    job_details.set_status(JobState::Killed);
    storage.update_by_id(job_id.clone(),&job_details).await.unwrap();
    let response =  format!("Job {} killed",job_id.clone());
    HttpResponse::Ok().body(response)
}

pub async fn get_workers(storage: web::Data<PostgresStorage<TrainJob>>) -> HttpResponse
{
    let storage = &*storage.into_inner();
    let mut storage = storage.clone();
    let workers = storage.list_workers().await;
    match workers {
        Ok(workers) => HttpResponse::Ok().json(serde_json::to_value(workers).unwrap()),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}

pub async fn get_job(job_id: web::Path<String>, storage: web::Data<PostgresStorage<TrainJob>>) -> HttpResponse
{
    let storage = &*storage.into_inner();
    let storage = storage.clone();
    let res = storage.fetch_by_id(job_id.clone()).await;
    match res {
        Ok(Some(job)) => HttpResponse::Ok().json(job),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("{e}")),
    }
}