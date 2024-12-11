use sea_orm::{EnumIter, Iterable};
use sea_orm_migration::{prelude::*, schema::*};
use sea_query::extension::postgres::Type;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(Alias::new("status"))
                    .values(Status::iter())
                    .to_owned()
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Videos::Table)
                    .if_not_exists()
                    .col(pk_auto(Videos::Id))
                    .col(string(Videos::Title))
                    .col(string_uniq(Videos::YtId))
                    .col(timestamp(Videos::CreatedAt))
                    .col(enumeration(Videos::Status, Alias::new("status"), Status::iter()))
                    .col(timestamp_null(Videos::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Actions::Table)
                    .if_not_exists()
                    .col(pk_auto(Actions::Id))
                    .col(string_len(Actions::Uid, 36))
                    .col(integer(Actions::Vid))
                    .col(timestamp(Actions::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_videos_id")
                            .from(Actions::Table, Actions::Vid)
                            .to(Videos::Table, Videos::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Moderators::Table)
                    .if_not_exists()
                    .col(string_len_uniq(Moderators::Uid, 36).primary_key())
                    .col(timestamp(Moderators::CreatedAt))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Actions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Videos::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Moderators::Table).to_owned())
            .await?;
        manager
            .drop_type(
            Type::drop().if_exists().name(Alias::new("status")).cascade().to_owned())
            .await?;
        // manager
        //     .drop_foreign_key(
        //     ForeignKey::drop().name("fk_videos_id").to_owned())
        //     .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Videos {
    Table,
    Id,
    Title,
    YtId,
    CreatedAt,
    Status,
    UpdatedAt

}

#[derive(DeriveIden)]
enum Actions {
    Table,
    Id,
    Vid,
    Uid,
    CreatedAt
}

#[derive(Iden, EnumIter)]
pub enum Status {
    #[iden = "Pending"]
    Pending,
    #[iden = "Viewed"]
    Viewed,
    #[iden = "Banned"]
    Banned
}

#[derive(DeriveIden)]
enum Moderators {
    Table,
    Uid,
    CreatedAt,
}