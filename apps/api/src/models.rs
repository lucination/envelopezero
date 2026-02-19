use chrono::NaiveDateTime;
use derive_builder::Builder;
use pillid::pillid;
use serde::Deserialize;
use serde::Serialize;

pub type UserPillid = String;
pub type BudgetPillid = String;
pub type BudgetIdemPillid = String;
pub type SupercategoryPillid = String;
pub type CategoryPillid = String;
pub type PayeePillid = String;
pub type AccountPillid = String;
pub type TransactionPillid = String;
pub type TransactionDetailPillid = String;
pub type AccessTokenPillid = String;

pub fn new_pillid() -> String {
    pillid!().to_string()
}

pub trait BaseModel {
    fn pillid(&self) -> &str;
    fn created_at(&self) -> NaiveDateTime;
    fn updated_at(&self) -> NaiveDateTime;
    fn deleted_at(&self) -> Option<NaiveDateTime>;
}

pub trait UserModel {
    fn user_pillid(&self) -> &str;
}

#[derive(Debug, Serialize, Deserialize, Default, Builder, PartialEq, Eq, PartialOrd, Ord)]
#[builder(pattern = "owned", default)]
pub struct Budget {
    pub pillid: BudgetPillid,
    pub user_pillid: UserPillid,
    pub idem_pillid: BudgetIdemPillid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for Budget {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for Budget {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct Category {
    pub pillid: CategoryPillid,
    pub user_pillid: UserPillid,
    pub budget_pillid: BudgetPillid,
    pub supercategory_pillid: SupercategoryPillid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for Category {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for Category {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct Payee {
    pub pillid: PayeePillid,
    pub user_pillid: UserPillid,
    pub budget_pillid: BudgetPillid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for Payee {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for Payee {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct Supercategory {
    pub pillid: SupercategoryPillid,
    pub user_pillid: UserPillid,
    pub budget_pillid: BudgetPillid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for Supercategory {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for Supercategory {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct Transaction {
    pub pillid: TransactionPillid,
    pub user_pillid: UserPillid,
    pub budget_pillid: BudgetPillid,
    pub account_pillid: AccountPillid,
    pub payee_pillid: PayeePillid,
    pub name: String,
    pub transaction_date: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for Transaction {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for Transaction {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct TransactionDetail {
    pub pillid: TransactionDetailPillid,
    pub user_pillid: UserPillid,
    pub transaction_pillid: TransactionPillid,
    pub budget_pillid: BudgetPillid,
    pub category_pillid: CategoryPillid,
    pub memo: Option<String>,
    pub inflow: i64,
    pub outflow: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for TransactionDetail {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for TransactionDetail {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder, PartialEq, Eq, PartialOrd, Ord)]
#[builder(pattern = "owned", default)]
pub struct User {
    pub pillid: UserPillid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for User {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct Account {
    pub pillid: AccountPillid,
    pub user_pillid: UserPillid,
    pub budget_pillid: BudgetPillid,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for Account {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for Account {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct AccessToken {
    pub pillid: AccessTokenPillid,
    pub user_pillid: UserPillid,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl BaseModel for AccessToken {
    fn pillid(&self) -> &str {
        &self.pillid
    }
    fn created_at(&self) -> NaiveDateTime {
        self.created_at
    }
    fn updated_at(&self) -> NaiveDateTime {
        self.updated_at
    }
    fn deleted_at(&self) -> Option<NaiveDateTime> {
        self.deleted_at
    }
}

impl UserModel for AccessToken {
    fn user_pillid(&self) -> &str {
        &self.user_pillid
    }
}
