use pushkind_common::repository::errors::RepositoryError;
use pushkind_template::domain::template::{NewTemplate, UpdateTemplate};
use pushkind_template::repository::DieselRepository;
use pushkind_template::repository::{TemplateListQuery, TemplateReader, TemplateWriter};

mod common;

#[test]
fn test_template_repository_crud() {
    let test_db = common::TestDb::new("test_template_repository_crud.db");
    let template_repo = DieselRepository::new(test_db.pool());
    let c1 = NewTemplate::new(Some("Alice".to_string()), 1);
    let c2 = NewTemplate::new(Some("Bobby".to_string()), 1);

    assert_eq!(template_repo.create_templates(&[c1, c2]).unwrap(), 2);

    let (total, mut items) = template_repo
        .list_templates(TemplateListQuery::new(1))
        .unwrap();
    assert_eq!(total, 2);
    assert_eq!(items.len(), 2);
    items.sort_by(|a, b| a.value.cmp(&b.value));
    let mut alice = items[0].clone();

    alice = template_repo
        .update_template(alice.id, 1, &UpdateTemplate::new(Some("alice".to_string())))
        .unwrap();
    assert_eq!(alice.value, Some("alice".to_string()));

    let err = template_repo
        .update_template(
            alice.id,
            2,
            &UpdateTemplate::new(Some("intruder".to_string())),
        )
        .err()
        .expect("expected hub-scoped update to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    let err = template_repo
        .delete_template(alice.id, 2)
        .expect_err("expected hub-scoped delete to fail");
    assert!(matches!(err, RepositoryError::NotFound));

    template_repo.delete_template(alice.id, 1).unwrap();
    assert!(
        template_repo
            .get_template_by_id(alice.id, 1)
            .unwrap()
            .is_none()
    );

    let (total_after, items_after) = template_repo
        .list_templates(TemplateListQuery::new(1))
        .unwrap();
    assert_eq!(total_after, 1);
    assert_eq!(items_after[0].value, Some("Bobby".to_string()));
}
