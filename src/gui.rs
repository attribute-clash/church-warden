use chrono::Local;
use eframe::egui;
use rusqlite::params;

use crate::{
    Database,
    models::{InventoryReceiptItem, Role, SaleItem, User},
    modules::{
        admin::AdminService,
        auth::AuthService,
        backup::{BackupService, BackupTransport},
        reports::{ReportsService, TurnoverRow},
        shop::ShopService,
        warehouse::WarehouseService,
    },
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Shop,
    Warehouse,
    Reports,
    Admin,
}

pub struct ChurchWardenApp {
    db: Database,
    current_user: Option<User>,
    active: Section,
    login: String,
    password: String,
    qr_token: String,
    login_error: String,

    sale_product_code: String,
    sale_qty: i64,
    warehouse_product_code: String,
    warehouse_product_name: String,
    warehouse_qty: i64,
    warehouse_price: f64,

    day_to_close: String,
    report_rows: Vec<TurnoverRow>,

    backup_transport: BackupTransport,
    backup_target: String,
    backup_message: String,
}

impl ChurchWardenApp {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            current_user: None,
            active: Section::Shop,
            login: String::new(),
            password: String::new(),
            qr_token: String::new(),
            login_error: String::new(),
            sale_product_code: String::new(),
            sale_qty: 1,
            warehouse_product_code: String::new(),
            warehouse_product_name: String::new(),
            warehouse_qty: 1,
            warehouse_price: 0.0,
            day_to_close: Local::now().format("%Y-%m-%d").to_string(),
            report_rows: vec![],
            backup_transport: BackupTransport::Scp,
            backup_target: String::new(),
            backup_message: String::new(),
        }
    }

    fn visible_sections(&self) -> Vec<Section> {
        let Some(user) = &self.current_user else {
            return vec![];
        };

        let admin = AdminService::new(&self.db);
        let names = admin.visible_sections(user.id).unwrap_or_default();
        let mut out = vec![];
        for name in names {
            match name.as_str() {
                "Лавка" => out.push(Section::Shop),
                "Склад" => out.push(Section::Warehouse),
                "Отчёты" => out.push(Section::Reports),
                "Администрирование" => out.push(Section::Admin),
                _ => {}
            }
        }
        out
    }

    fn section_name(section: Section) -> &'static str {
        match section {
            Section::Shop => "Лавка",
            Section::Warehouse => "Склад",
            Section::Reports => "Отчёты",
            Section::Admin => "Администрирование",
        }
    }

    fn ensure_seed_admin(&self) {
        let auth = AuthService::new(&self.db);
        let _ = auth.create_user("admin", "admin123", "QR:ADMIN", Role::Admin);
    }
}

impl eframe::App for ChurchWardenApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_seed_admin();

        if self.current_user.is_none() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Вход в Church Warden");
                ui.separator();
                ui.label("Вход по логину/паролю");
                ui.horizontal(|ui| {
                    ui.label("Логин");
                    ui.text_edit_singleline(&mut self.login);
                });
                ui.horizontal(|ui| {
                    ui.label("Пароль");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                });
                if ui.button("Войти").clicked() {
                    let auth = AuthService::new(&self.db);
                    match auth.login_with_password(&self.login, &self.password) {
                        Ok(user) => {
                            self.current_user = Some(user);
                            self.login_error.clear();
                        }
                        Err(e) => self.login_error = format!("Ошибка входа: {e}"),
                    }
                }

                ui.separator();
                ui.label("Вход по персональному QR");
                ui.horizontal(|ui| {
                    ui.label("QR token");
                    ui.text_edit_singleline(&mut self.qr_token);
                    if ui.button("Сканировать/Войти").clicked() {
                        let auth = AuthService::new(&self.db);
                        match auth.login_with_qr(&self.qr_token) {
                            Ok(user) => {
                                self.current_user = Some(user);
                                self.login_error.clear();
                            }
                            Err(e) => self.login_error = format!("Ошибка QR-входа: {e}"),
                        }
                    }
                });

                if !self.login_error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &self.login_error);
                }
            });
            return;
        }

        let user = self.current_user.clone().expect("checked above");
        let visible = self.visible_sections();

        egui::SidePanel::left("left_menu")
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading(format!("Пользователь: {}", user.username));
                ui.separator();
                for section in &visible {
                    if ui
                        .selectable_label(self.active == *section, Self::section_name(*section))
                        .clicked()
                    {
                        self.active = *section;
                    }
                }
                ui.separator();
                if ui.button("Выход").clicked() {
                    self.current_user = None;
                    self.login.clear();
                    self.password.clear();
                    self.qr_token.clear();
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.active {
            Section::Shop => {
                ui.heading("Лавка");
                ui.horizontal(|ui| {
                    ui.label("Код товара (или QR)");
                    ui.text_edit_singleline(&mut self.sale_product_code);
                    ui.label("Кол-во");
                    ui.add(egui::DragValue::new(&mut self.sale_qty).range(1..=1000));
                    if ui.button("Провести продажу").clicked() {
                        let product_id = self
                            .db
                            .conn()
                            .query_row(
                                "SELECT id FROM products WHERE code=?1",
                                params![self.sale_product_code.clone()],
                                |r| r.get::<_, i64>(0),
                            )
                            .ok();
                        if let Some(product_id) = product_id {
                            let shop = ShopService::new(&self.db);
                            let _ = shop.register_sale(
                                user.id,
                                &Local::now().format("%Y-%m-%d").to_string(),
                                &[SaleItem {
                                    product_id,
                                    qty: self.sale_qty,
                                }],
                            );
                        }
                    }
                });
                ui.separator();
                ui.label("Последние продажи");
                egui::Grid::new("sales_table").striped(true).show(ui, |ui| {
                    ui.strong("ID");
                    ui.strong("Дата");
                    ui.strong("Сумма");
                    ui.end_row();

                    let mut stmt = self
                        .db
                        .conn()
                        .prepare(
                            "SELECT id, created_at, total FROM sales ORDER BY id DESC LIMIT 20",
                        )
                        .expect("prepare sales");
                    let rows = stmt
                        .query_map([], |row| {
                            Ok((
                                row.get::<_, i64>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, f64>(2)?,
                            ))
                        })
                        .expect("query sales");
                    for row in rows.flatten() {
                        ui.label(row.0.to_string());
                        ui.label(row.1);
                        ui.label(format!("{:.2}", row.2));
                        ui.end_row();
                    }
                });
            }
            Section::Warehouse => {
                ui.heading("Склад");
                ui.horizontal(|ui| {
                    ui.label("Код");
                    ui.text_edit_singleline(&mut self.warehouse_product_code);
                    ui.label("Наименование");
                    ui.text_edit_singleline(&mut self.warehouse_product_name);
                });
                ui.horizontal(|ui| {
                    ui.label("Кол-во");
                    ui.add(egui::DragValue::new(&mut self.warehouse_qty).range(1..=10000));
                    ui.label("Цена");
                    ui.add(egui::DragValue::new(&mut self.warehouse_price).speed(1.0));
                    if ui.button("Оприходовать").clicked() {
                        let warehouse = WarehouseService::new(&self.db);
                        let _ = warehouse.register_receipt(
                            user.id,
                            &[InventoryReceiptItem {
                                product_code: self.warehouse_product_code.clone(),
                                product_name: self.warehouse_product_name.clone(),
                                qty: self.warehouse_qty,
                                price: self.warehouse_price,
                            }],
                        );
                    }
                });

                ui.separator();
                ui.label("Текущие остатки");
                egui::Grid::new("products_table")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Код");
                        ui.strong("Наименование");
                        ui.strong("Остаток");
                        ui.strong("Цена");
                        ui.end_row();

                        let mut stmt = self
                            .db
                            .conn()
                            .prepare(
                                "SELECT code, name, stock_qty, price FROM products ORDER BY code",
                            )
                            .expect("prepare products");
                        let rows = stmt
                            .query_map([], |row| {
                                Ok((
                                    row.get::<_, String>(0)?,
                                    row.get::<_, String>(1)?,
                                    row.get::<_, i64>(2)?,
                                    row.get::<_, f64>(3)?,
                                ))
                            })
                            .expect("query products");
                        for row in rows.flatten() {
                            ui.label(row.0);
                            ui.label(row.1);
                            ui.label(row.2.to_string());
                            ui.label(format!("{:.2}", row.3));
                            ui.end_row();
                        }
                    });
            }
            Section::Reports => {
                ui.heading("Отчёты");
                ui.horizontal(|ui| {
                    if ui.button("Построить ОСВ").clicked() {
                        let reports = ReportsService::new(&self.db);
                        self.report_rows = reports.turnover_report().unwrap_or_default();
                    }
                    ui.label("День для закрытия");
                    ui.text_edit_singleline(&mut self.day_to_close);
                    if ui.button("Закрыть день").clicked() {
                        let reports = ReportsService::new(&self.db);
                        let _ = reports.close_day(user.id, &self.day_to_close);
                    }
                });

                ui.separator();
                egui::Grid::new("report_table")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Код");
                        ui.strong("Приход");
                        ui.strong("Расход");
                        ui.strong("Остаток");
                        ui.end_row();
                        for row in &self.report_rows {
                            ui.label(&row.product_code);
                            ui.label(row.incoming_qty.to_string());
                            ui.label(row.outgoing_qty.to_string());
                            ui.label(row.closing_qty.to_string());
                            ui.end_row();
                        }
                    });
            }
            Section::Admin => {
                ui.heading("Администрирование");
                ui.label("Пользователи и роли");
                egui::Grid::new("users_table").striped(true).show(ui, |ui| {
                    ui.strong("ID");
                    ui.strong("Логин");
                    ui.strong("Роль");
                    ui.strong("Активен");
                    ui.end_row();

                    let mut stmt = self
                        .db
                        .conn()
                        .prepare("SELECT id, username, role, is_active FROM users ORDER BY id")
                        .expect("prepare users");
                    let rows = stmt
                        .query_map([], |row| {
                            Ok((
                                row.get::<_, i64>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                                row.get::<_, i64>(3)?,
                            ))
                        })
                        .expect("query users");
                    for row in rows.flatten() {
                        ui.label(row.0.to_string());
                        ui.label(row.1);
                        ui.label(row.2);
                        ui.label(if row.3 == 1 { "Да" } else { "Нет" });
                        ui.end_row();
                    }
                });

                ui.separator();
                ui.label("Резервное копирование БД по SSH (SCP/SFTP)");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.backup_transport, BackupTransport::Scp, "SCP");
                    ui.selectable_value(&mut self.backup_transport, BackupTransport::Sftp, "SFTP");
                });
                ui.horizontal(|ui| {
                    ui.label("Удалённый target (пример user@host:/path)");
                    ui.text_edit_singleline(&mut self.backup_target);
                });
                if ui.button("Сделать backup church_warden.db").clicked() {
                    self.backup_message = match BackupService::run_backup(
                        "church_warden.db",
                        &self.backup_target,
                        self.backup_transport,
                    ) {
                        Ok(_) => "Backup завершён успешно".to_string(),
                        Err(e) => format!("Ошибка backup: {e}"),
                    };
                }
                if !self.backup_message.is_empty() {
                    ui.label(&self.backup_message);
                }
            }
        });
    }
}
