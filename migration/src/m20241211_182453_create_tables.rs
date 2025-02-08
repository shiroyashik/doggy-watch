use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Videos
        manager
            .create_table(
                Table::create()
                    .table(Videos::Table)
                    .if_not_exists()
                    .col(string_len_uniq(Videos::Ytid, 11).primary_key())
                    .col(string(Videos::Title))
                    .col(boolean(Videos::Banned).default(Expr::value(false)))
                    .to_owned(),
            )
            .await?;
        // Requests
        manager
            .create_table(
                Table::create()
                    .table(Requests::Table)
                    .if_not_exists()
                    .col(pk_auto(Requests::Id))
                    .col(string_len(Requests::Ytid, 11))
                    .col(timestamp_null(Requests::ViewedAt).default(Expr::value(Keyword::Null)))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_videos_ytid_requests")
                            .from(Requests::Table, Requests::Ytid)
                            .to(Videos::Table, Videos::Ytid)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;
        // Actions
        manager
            .create_table(
                Table::create()
                    .table(Actions::Table)
                    .if_not_exists()
                    .col(pk_auto(Actions::Id))
                    .col(integer(Actions::Rid))
                    .col(big_integer(Actions::Uid))
                    .col(timestamp(Actions::CreatedAt).default(Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_requests_rid_actions")
                            .from(Actions::Table, Actions::Rid)
                            .to(Requests::Table, Requests::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;
        // Archived
        manager
            .create_table(
                Table::create()
                    .table(Archived::Table)
                    .if_not_exists()
                    .col(pk_auto(Archived::Id))
                    .col(string_len(Archived::Ytid, 11))
                    .col(timestamp_null(Archived::ViewedAt))
                    .col(big_integer(Archived::CreatedBy))
                    .col(timestamp(Archived::CreatedAt).default(Expr::current_timestamp()))
                    .col(unsigned(Archived::Contributors))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_videos_ytid_archived")
                            .from(Archived::Table, Archived::Ytid)
                            .to(Videos::Table, Videos::Ytid)
                            .on_delete(ForeignKeyAction::NoAction)
                    )
                    .to_owned(),
            )
            .await?;
        // Moderators
        manager
            .create_table(
                Table::create()
                    .table(Moderators::Table)
                    .if_not_exists()
                    .col(big_integer_uniq(Moderators::Id).primary_key())
                    .col(timestamp(Moderators::CreatedAt).default(Expr::current_timestamp()))
                    .col(boolean(Moderators::Notify).default(Expr::value(true)))
                    .col(boolean(Moderators::CanAddMods).default(Expr::value(false)))
                    .to_owned(),
            )
            .await?;
        // Users
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(big_integer_uniq(Users::Id).primary_key())
                    .col(timestamp(Users::CreatedAt).default(Expr::current_timestamp()))
                    .col(unsigned(Users::Contributions).default(Expr::value(0)))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Actions
        manager
            .drop_table(Table::drop().table(Actions::Table).to_owned())
            .await?;
        // Requests
        manager
            .drop_table(Table::drop().table(Requests::Table).to_owned())
            .await?;
        // Archived
        manager
            .drop_table(Table::drop().table(Archived::Table).to_owned())
            .await?;
        // Videos
        manager
            .drop_table(Table::drop().table(Videos::Table).to_owned())
            .await?;
        // Moderators
        manager
            .drop_table(Table::drop().table(Moderators::Table).to_owned())
            .await?;
        // Users
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Videos {
    Table,
    Ytid,
    Title,
    Banned
}

#[derive(DeriveIden)]
enum Requests {
    Table,
    Id,
    Ytid,
    ViewedAt
}

#[derive(DeriveIden)]
enum Actions {
    Table,
    Id,
    Rid,
    Uid,
    CreatedAt
}

#[derive(DeriveIden)]
enum Archived {
    Table,
    Id,
    Ytid,
    ViewedAt,
    CreatedBy,
    CreatedAt,
    Contributors
}

#[derive(DeriveIden)]
enum Moderators {
    Table,
    Id,
    CreatedAt,
    Notify,
    CanAddMods
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    CreatedAt,
    Contributions
}