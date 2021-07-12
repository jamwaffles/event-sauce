use event_sauce::*;
use std::convert::TryInto;
use uuid::Uuid;

#[test]
fn create() {
    // ---
    // Storage provider "crate"
    // ---

    #[derive(Debug)]
    struct FakeStorage {
        items: Vec<User>,
        events: Vec<DBEvent>,
    }

    impl Storage for FakeStorage {
        type Error = ();
    }

    impl<D, E> Persistable<FakeStorage, E> for Persister<D, E>
    where
        D: EventData,
        E: Entity + Persistable<FakeStorage>,
    {
        fn persist(self, storage: &mut FakeStorage) -> Result<E, <FakeStorage as Storage>::Error> {
            let event: DBEvent = self.event.try_into().expect("Failed to convert event");

            storage.events.push(event);

            self.entity.persist(storage)
        }
    }

    // ---
    // Client implementation
    // ---

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct User {
        name: String,
        email: String,
    }

    impl Entity for User {
        const ENTITY_TYPE: &'static str = "users";

        fn entity_id(&self) -> Uuid {
            Uuid::nil()
        }
    }

    impl Persistable<FakeStorage> for User {
        fn persist(
            self,
            storage: &mut FakeStorage,
        ) -> Result<Self, <FakeStorage as Storage>::Error> {
            storage.items.push(self.clone());

            Ok(self)
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UserCreated {
        name: String,
        email: String,
    }

    impl EventData for UserCreated {
        type Entity = User;

        fn event_type() -> &'static str {
            "UserCreated"
        }
    }

    impl Create<UserCreated> for User {
        fn create_from(event: &Event<UserCreated>) -> Self {
            Self {
                name: event.data.name.clone(),
                email: event.data.email.clone(),
            }
        }
    }

    let mut storage = FakeStorage {
        items: Vec::new(),
        events: Vec::new(),
    };

    let user = User::create(UserCreated {
        name: "Foo Bar".to_string(),
        email: "foo@bar.com".to_string(),
    })
    .persist(&mut storage)
    .unwrap();

    dbg!(user);

    dbg!(storage);
}

#[test]
fn event_builder() {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct User {
        name: String,
        email: String,
    }

    impl Entity for User {
        const ENTITY_TYPE: &'static str = "users";

        fn entity_id(&self) -> Uuid {
            Uuid::nil()
        }
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UserCreated {
        name: String,
        email: String,
    }

    impl EventData for UserCreated {
        type Entity = User;

        fn event_type() -> &'static str {
            "UserCreated"
        }
    }

    let entity_id = Uuid::new_v4();

    let event = UserCreated {
        name: String::new(),
        email: String::new(),
    }
    .into_builder()
    .entity_id(entity_id)
    .build();

    assert_eq!(event.entity_id, entity_id);
}
