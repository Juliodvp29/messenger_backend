use crate::repositories::contact::PostgresContactRepository;
use domain::contact::entity::Contact;
use domain::contact::repository::ContactRepository;
use domain::user::value_objects::PhoneNumber;
use domain::user::value_objects::UserId;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_contact(pool: PgPool) -> sqlx::Result<()> {
    let repo = PostgresContactRepository::new(pool.clone(), None);
    let owner_id = UserId(uuid::Uuid::new_v4());

    // Create the owner user first to avoid FK violation
    sqlx::query("INSERT INTO users (id, phone, phone_hash) VALUES ($1, $2, $3)")
        .bind(owner_id.0)
        .bind("+573000000001")
        .bind("dummy_hash_1")
        .execute(&pool)
        .await?;

    let phone = PhoneNumber::new("+573001111111".to_string()).unwrap();

    let contact = Contact::new(owner_id.clone(), phone.clone(), None);

    repo.create(&contact).await.expect("Should create contact");

    let found = repo
        .find_by_owner_and_phone(&owner_id, &phone)
        .await
        .expect("Should find contact");
    assert!(found.is_some());
    assert_eq!(found.unwrap().phone.as_str(), "+573001111111");

    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_all_by_owner(pool: PgPool) -> sqlx::Result<()> {
    let repo = PostgresContactRepository::new(pool.clone(), None);
    let owner_id = UserId(uuid::Uuid::new_v4());

    // Create the owner user first to avoid FK violation
    sqlx::query("INSERT INTO users (id, phone, phone_hash) VALUES ($1, $2, $3)")
        .bind(owner_id.0)
        .bind("+573000000002")
        .bind("dummy_hash_2")
        .execute(&pool)
        .await?;

    let phone1 = PhoneNumber::new("+573001111111".to_string()).unwrap();
    let phone2 = PhoneNumber::new("+573001222222".to_string()).unwrap();

    let contact1 = Contact::new(owner_id.clone(), phone1, None);
    let contact2 = Contact::new(owner_id.clone(), phone2, None);

    repo.create(&contact1)
        .await
        .expect("Should create contact1");
    repo.create(&contact2)
        .await
        .expect("Should create contact2");

    let all = repo
        .find_all_by_owner(&owner_id)
        .await
        .expect("Should find all contacts");
    assert_eq!(all.len(), 2);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_contact(pool: PgPool) -> sqlx::Result<()> {
    let repo = PostgresContactRepository::new(pool.clone(), None);
    let owner_id = UserId(uuid::Uuid::new_v4());

    // Create the owner user first to avoid FK violation
    sqlx::query("INSERT INTO users (id, phone, phone_hash) VALUES ($1, $2, $3)")
        .bind(owner_id.0)
        .bind("+573000000003")
        .bind("dummy_hash_3")
        .execute(&pool)
        .await?;

    let phone = PhoneNumber::new("+573001333333".to_string()).unwrap();

    let contact = Contact::new(owner_id.clone(), phone.clone(), None);

    repo.create(&contact).await.expect("Should create contact");

    let mut found = repo
        .find_by_owner_and_phone(&owner_id, &phone)
        .await
        .expect("Should find")
        .unwrap();
    found.set_favorite(true);
    repo.update(&found).await.expect("Should update");

    let updated = repo
        .find_by_owner_and_phone(&owner_id, &phone)
        .await
        .expect("Should find")
        .unwrap();
    assert!(updated.is_favorite);

    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_contact(pool: PgPool) -> sqlx::Result<()> {
    let repo = PostgresContactRepository::new(pool.clone(), None);
    let owner_id = UserId(uuid::Uuid::new_v4());

    // Create the owner user first to avoid FK violation
    sqlx::query("INSERT INTO users (id, phone, phone_hash) VALUES ($1, $2, $3)")
        .bind(owner_id.0)
        .bind("+573000000004")
        .bind("dummy_hash_4")
        .execute(&pool)
        .await?;

    let phone = PhoneNumber::new("+573001444444".to_string()).unwrap();

    let contact = Contact::new(owner_id.clone(), phone.clone(), None);

    repo.create(&contact).await.expect("Should create contact");

    let found = repo
        .find_by_owner_and_phone(&owner_id, &phone)
        .await
        .expect("Should find")
        .unwrap();
    repo.delete(&found.id).await.expect("Should delete");

    let deleted = repo.find_by_id(&found.id).await.expect("Should not find");
    assert!(deleted.is_none());

    Ok(())
}
