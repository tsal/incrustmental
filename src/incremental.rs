use std::time::Duration;

use rand::Rng;

use derive_getters::Getters;

use crate::prelude::AutomationId;
use crate::types::{Count, PerkId, Price, ProductId, ProductMaterialId};

use crate::timer::Timer;

#[derive(Copy, Clone, Debug)]
pub enum Quantity {
    Money(Price),
    Material(ProductMaterialId, Count),
    Product(ProductId, Count),
}

impl Quantity {
    pub fn similar(&self, other: &Quantity) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn op(&self, other: &Quantity, op: fn(f64, f64) -> f64) -> Quantity {
        match (self, other) {
            (Quantity::Money(x), Quantity::Money(y)) => Quantity::Money(op(*x, *y)),
            (Quantity::Material(x, y), Quantity::Material(z, w)) => {
                if x == z {
                    Quantity::Material(*x, op(*y as f64, *w as f64) as Count)
                } else {
                    *self
                }
            }
            (Quantity::Product(x, y), Quantity::Product(z, w)) => {
                if x == z {
                    Quantity::Product(*x, op(*y as f64, *w as f64) as Count)
                } else {
                    *self
                }
            }
            _ => *self,
        }
    }

    pub fn quantity(&self) -> f64 {
        match &self {
            Quantity::Money(x) => *x,
            Quantity::Material(_, x) => *x as f64,
            Quantity::Product(_, x) => *x as f64,
        }
    }

    pub fn as_str(&self, state: &State) -> String {
        match &self {
            Quantity::Money(x) => format!("${:.2}", *x),
            Quantity::Material(id, cnt) => format!(
                "{} {}{}",
                *cnt,
                state.materials[*id].name.to_lowercase(),
                if *cnt > 1 { "s" } else { "" }
            ),
            Quantity::Product(id, cnt) => format!(
                "{} {}{}",
                *cnt,
                state.products[*id].name.to_lowercase(),
                if *cnt > 1 { "s" } else { "" }
            ),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum RelationKind {
    Consumes,
    ManifacturedBy,
    SoldBy,
    Needs,
}

#[derive(Copy, Clone, Debug)]
pub struct Relation {
    kind: RelationKind,
    quantity: Quantity,
}

impl Relation {
    pub fn new(kind: RelationKind, quantity: Quantity) -> Self {
        Self { kind, quantity }
    }

    pub fn needs(quantity: Quantity) -> Self {
        Self {
            kind: RelationKind::Needs,
            quantity,
        }
    }

    pub fn consumes(quantity: Quantity) -> Self {
        Self {
            kind: RelationKind::Consumes,
            quantity,
        }
    }

    pub fn kind(&self) -> RelationKind {
        self.kind
    }

    pub fn quantity(&self) -> &Quantity {
        &self.quantity
    }

    pub fn similar_quantity(&self, other: &Quantity) -> bool {
        self.quantity.similar(other)
    }

    fn add(&self, perk: Quantity) -> Relation {
        let mut new_cond = *self;
        new_cond.quantity = self.quantity.op(&perk, |x, y| x + y);
        new_cond
    }

    fn sub(&self, perk: Quantity) -> Relation {
        let mut new_cond = *self;
        new_cond.quantity = self.quantity.op(&perk, |x, y| x - y);
        new_cond
    }

    fn multiply(&self, perk: Quantity) -> Relation {
        let mut new_cond = *self;
        new_cond.quantity = self.quantity.op(&perk, |x, y| x * y);
        new_cond
    }

    fn divide(&self, perk: Quantity) -> Relation {
        let mut new_cond = *self;
        new_cond.quantity = self.quantity.op(&perk, |x, y| x / y);
        new_cond
    }
}

// Materials that can be bought.
// Each material has a base price and a growth factor that
// determine the current price of the material.
// Materials used during the manifacturing or selling of a product.
// F.e ProductMaterial may be a Shop which sells certain product.
// In that case the product will have a dependancy to that material
// with RelationKind::SoldBy.
pub struct ProductMaterial {
    base_price: Price,
    bought: Count,
    count: Count,
    name: String,
    growth: f64,
    active: bool, // wether or not the product unlocked for the player
}

impl ProductMaterial {
    pub fn new(init_bought: Count, kind: String, base_price: Price, growth: f64, unlocked: bool) -> Self {
        Self {
            bought: init_bought,
            name: kind,
            count: init_bought,
            base_price,
            growth,
            active: unlocked,
        }
    }

    pub fn price(&self) -> Price {
        self.base_price * self.growth.powf((self.bought as f64) / 10.0)
    }

    pub(crate) fn buy(&mut self) {
        self.bought += 1;
        self.count += 1;
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn growth(&self) -> f64 {
        self.growth
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn count(&self) -> Count {
        self.count
    }

    pub(crate) fn activate(&mut self) {
        self.active = true;
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum PerkKind {
    Add,
    Substract,
    Multiply,
    Divide,
}

pub struct Perk {
    name: String,
    description: String,
    condition: Vec<Quantity>, // Always a Needs relation
    buy_price: Vec<Quantity>, // Always a Consume relation
    perk: (Quantity, PerkKind),
    unlocked: bool,
    active: bool,
}

impl Perk {
    pub fn new(
        name: String,
        description: String,
        condition: Vec<Quantity>,
        buy_price: Vec<Quantity>,
        perk: (Quantity, PerkKind),
    ) -> Self {
        Self {
            name,
            description,
            condition: condition,
            buy_price,
            perk,
            unlocked: false,
            active: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn condition(&self) -> &Vec<Quantity> {
        &self.condition
    }

    pub fn price(&self) -> &Vec<Quantity> {
        &self.buy_price
    }

    pub fn unlocked(&self) -> bool {
        self.unlocked
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub(crate) fn perk(&self) -> (Quantity, PerkKind) {
        self.perk
    }

    pub(crate) fn unlock(&mut self) {
        self.unlocked = true;
    }

    pub(crate) fn activate(&mut self) {
        self.active = true;
    }
}

pub struct Product {
    name: String,
    count: Count,
    price: Option<Price>,
    sold: Count,
    dependencies: Vec<Relation>,
    perks: Vec<PerkId>,
    unlocks: Vec<(ProductId, Count)>,
    active: bool,
}

impl Product {
    pub fn new(
        name: String,
        price: Option<Price>,
        dependencies: Vec<Relation>,
        perks: Vec<PerkId>,
        unlocks: Vec<(ProductId, Count)>,
        unlocked: bool
    ) -> Self {
        Self {
            name,
            count: 0,
            price,
            sold: 0,
            dependencies,
            perks,
            unlocks,
            active: unlocked,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn count(&self) -> Count {
        self.count
    }

    pub fn sold(&self) -> Count {
        self.count
    }

    pub fn interest(&self) -> f64 {
        match self.price {
            None => 0.0,
            Some(price) => {
                assert!(price > 0.);
                let init = if price < 1.0 { 0.5 } else { 0.0 };

                let price = if price < 1.0 {
                    1.0 / (1.0 - price)
                } else {
                    price
                };

                init + 0.5 / price + ((self.sold as f64).powf(1.07) / 100.0)
            }
        }
    }

    pub fn price(&self) -> Option<Price> {
        self.price
    }

    pub fn recipe(&self, state: &State) -> String {
        let mut needs = Vec::new();
        let mut consumes = Vec::new();
        let mut manifactured_by = Vec::new();

        for rel in self.dependencies.iter() {
            match rel.kind {
                RelationKind::Consumes => consumes.push(rel.quantity()),
                RelationKind::ManifacturedBy => manifactured_by.push(rel.quantity()),
                RelationKind::Needs => needs.push(rel.quantity()),
                _ => {}
            }
        }

        let mut recipe = String::new();

        let mut has_prev = false;
        if !needs.is_empty() {
            recipe.push_str("Needs: ");
            for need in needs.iter() {
                recipe.push_str(&need.as_str(state));
            }
            has_prev = true;
        }

        if !consumes.is_empty() {
            if has_prev {
                recipe.push_str("; ");
            }

            recipe.push_str("Consumes: ");
            for consume in consumes.iter() {
                recipe.push_str(&consume.as_str(state));
            }

            has_prev = true;
        }

        if !manifactured_by.is_empty() {
            if has_prev {
                recipe.push_str("; ");
            }
            
            recipe.push_str("Manifactured by: ");
            for m in manifactured_by.iter() {
                recipe.push_str(&m.as_str(state));
            }
        }

        recipe
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub(crate) fn sell(&mut self, cnt: Count) {
        assert!(self.count >= cnt);

        self.sold += cnt;
        self.count -= cnt;
    }

    pub(crate) fn build(&mut self, cnt: Count) {
        self.count += cnt;
    }

    pub(crate) fn dependencies(&self) -> &Vec<Relation> {
        &self.dependencies
    }

    pub(crate) fn perks(&self) -> &Vec<PerkId> {
        &self.perks
    }

    pub(crate) fn unlocks(&self) -> &Vec<(ProductId, Count)> {
        &self.unlocks
    }

    pub(crate) fn activate(&mut self) {
        self.active = true;
    }
}

#[derive(Clone, Copy)]
pub enum AutomationKind {
    Buy(ProductMaterialId),
    Build(ProductId),
}

pub struct Automation {
    name: String,
    kind: AutomationKind,
    timer: Option<Timer>,
    condition: Vec<Quantity>,
    buy_price: Vec<Quantity>,
    paused: bool,
    unlocked: bool,
    active: bool,
}

impl Automation {
    pub fn new(
        name: String,
        kind: AutomationKind,
        timer: Option<Timer>,
        condition: Vec<Quantity>,
        buy_price: Vec<Quantity>,
    ) -> Self {
        Self {
            name,
            kind,
            timer,
            condition,
            buy_price,
            paused: false,
            unlocked: false,
            active: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self, state: &State) -> String {
        let time = if let Some(timer) = &self.timer {
            format!("every {:.2} seconds", timer.duration().as_secs_f64())
        } else {
            "continually".to_string()
        };

        match self.kind {
            AutomationKind::Buy(id) => {
                format!(
                    "Buys {}s {}!",
                    state.materials[id].name.to_lowercase(),
                    time
                )
            },
            AutomationKind::Build(id) => {
                format!(
                    "Builds {}s {}!",
                    state.products[id].name.to_lowercase(),
                    time
                )
            },
        }
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn unlocked(&self) -> bool {
        self.unlocked
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn price(&self) -> &Vec<Quantity> {
        &self.buy_price
    }

    pub(crate) fn kind(&self) -> AutomationKind {
        self.kind
    }

    pub(crate) fn timer(&mut self) -> Option<&mut Timer> {
        self.timer.as_mut()
    }

    pub(crate) fn condition(&self) -> &Vec<Quantity> {
        &self.condition
    }

    pub(crate) fn unlock(&mut self) {
        self.unlocked = true;
    }

    pub(crate) fn activate(&mut self) {
        self.active = true;
    }
}

pub struct Badge {
    name: String,
    description: String, // We let the designer write his custom description
    condition: Vec<Quantity>,
    unlocked: bool,
}

impl Badge {
    pub fn new(
        name: String,
        description: String,
        condition: Vec<Quantity>,
    ) -> Self {
        Self {
            name,
            description,
            condition,
            unlocked: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn unlocked(&self) -> bool {
        self.unlocked
    }

    pub(crate) fn condition(&self) -> &Vec<Quantity> {
        &self.condition
    }

    pub(crate) fn unlock(&mut self) {
        self.unlocked = true;
    }
}

#[derive(Default)]
pub struct Objective(Vec<Quantity>);

impl Objective {
    pub fn new(conds: Vec<Quantity>) -> Self {
        Self(conds)
    }

    pub fn win_condition(&self) -> &[Quantity] {
        &self.0
    }
}

#[derive(Default, Getters)]
pub struct State {
    #[getter(skip)]
    money: f64,
    objective: Objective,
    materials: Vec<ProductMaterial>,
    products: Vec<Product>,
    badges: Vec<Badge>,
    perks: Vec<Perk>,
    automations: Vec<Automation>,
    #[getter(skip)]
    win: bool,
}

impl State {
    pub fn new(
        init_money: Price,
        objective: Objective,
        materials: Vec<ProductMaterial>,
        products: Vec<Product>,
        badges: Vec<Badge>,
        perks: Vec<Perk>,
        automations: Vec<Automation>,
    ) -> Self {
        Self {
            money: init_money,
            objective,
            materials,
            products,
            badges,
            perks,
            automations,
            win: false,
        }
    }

    fn quantity_present_count(&self, q: &Quantity) -> Count {
        match q {
            Quantity::Money(money) => (self.money / *money).floor() as Count,
            Quantity::Material(id, cnt) => self.materials[*id].count / *cnt,
            Quantity::Product(id, cnt) => self.products[*id].sold / *cnt,
        }
    }

    fn check_conditions(&self, conds: &[Quantity]) -> bool {
        for cond in conds.iter() {
            if self.quantity_present_count(cond) == 0 {
                return false;
            }
        }

        true
    }

    fn activate_product(&mut self, id: ProductId) {
        self.products[id].activate();

        let mut activate_recursive = Vec::new();

        for dep in self.products[id].dependencies().iter() {
            match dep.quantity {
                Quantity::Money(_) => unreachable!(),
                Quantity::Material(id, _) => self.materials[id].activate(),
                Quantity::Product(id, _) => activate_recursive.push(id),
            }
        }

        for id in activate_recursive {
            self.activate_product(id);
        }
    }

    fn apply_perk(&self, id: ProductId, cond: Relation) -> Relation {
        let mut new_cond = cond;
        for perk_id in self.products[id].perks().iter() {
            let perk = &self.perks[*perk_id];
            if !perk.active {
                continue;
            }
            let perk = perk.perk();

            match &perk.1 {
                PerkKind::Add => new_cond = new_cond.add(perk.0),
                PerkKind::Substract => new_cond = new_cond.sub(perk.0),
                PerkKind::Multiply => new_cond = new_cond.multiply(perk.0),
                PerkKind::Divide => new_cond = new_cond.divide(perk.0),
            }
        }

        new_cond
    }

    fn apply_product_perks(&self, base_build_count: Count, id: ProductId) -> Count {
        let cond = Relation::needs(Quantity::Product(id, base_build_count));
        let cond = self.apply_perk(id, cond);

        if let Quantity::Product(product_id, cnt) = cond.quantity() {
            assert!(*product_id == id);

            *cnt
        } else {
            unreachable!()
        }
    }

    fn build_product_count(&mut self, id: ProductId) -> u64 {
        // Check conditions
        let mut prices = Vec::new();
        let mut max_buy_count = u64::MAX;
        let mut max_build_count = u64::MAX;
        for cond in self.products[id].dependencies().iter() {
            let cond = self.apply_perk(id, *cond);
            let cnt = self.quantity_present_count(cond.quantity());
            if cnt == 0 && cond.quantity().quantity() > 0.0 {
                return 0;
            }

            match cond.kind() {
                RelationKind::Consumes => {
                    max_buy_count = max_buy_count.min(cnt);
                    prices.push(*cond.quantity());
                }
                RelationKind::ManifacturedBy => {
                    max_build_count = max_build_count.min(cnt);
                }
                _ => {}
            }
        }

        let build_count = max_buy_count.min(max_build_count);

        // Buy the product
        for price in prices {
            match price {
                Quantity::Money(money) => {
                    assert!(self.money >= build_count as f64 * money);
                    self.money -= build_count as f64 * money;
                }
                Quantity::Material(id, cnt) => {
                    assert!(self.materials[id].count >= build_count * cnt);
                    self.materials[id].count -= build_count * cnt;
                }
                Quantity::Product(id, cnt) => {
                    assert!(self.products[id].count >= build_count * cnt);
                    self.products[id].count -= build_count * cnt;
                }
            }
        }

        // apply product perks so we know how much we can build at a time
        let built_cnt = self.apply_product_perks(build_count, id);

        built_cnt
    }

    fn sell_product(&mut self, id: ProductId) {
        let product = &self.products[id];

        if product.price().is_none() || product.interest() == 0.0 || product.count() == 0 {
            return;
        }

        let interest = product.interest().min(1.);
        let mut rng = rand::thread_rng();
        let sold = if rng.gen_bool(interest / 100.0) { 1 } else { 0 };
        if sold == 0 {
            return;
        }

        let mut sell_multiplier: Option<Count> = None;
        for dep in product.dependencies().iter() {
            if dep.kind() != RelationKind::SoldBy {
                continue;
            }
            let cnt = self.quantity_present_count(dep.quantity());
            if cnt == 0 {
                continue;
            }

            sell_multiplier = match sell_multiplier {
                Some(mult) => Some(mult.min(cnt)),
                None => Some(cnt),
            };
        }
        let sold = (sold * sell_multiplier.unwrap_or(1)).min(product.count());

        let product = &mut self.products[id];
        product.sell(sold);

        self.money += product.price().unwrap() * sold as f64;
    }

    pub fn construct_product(&mut self, id: ProductId) {
        let count = self.build_product_count(id);

        if count == 0 {
            return;
        }

        self.products[id].build(count);

        let mut products_to_activate = Vec::new();
        for (unlock_id, cnt) in self.products[id].unlocks() {
            if self.products[id].count > *cnt {
                products_to_activate.push(*unlock_id);
            }
        }

        for id in products_to_activate {
            self.activate_product(id);
        }
    }

    pub fn update(&mut self, delta: Duration) {
        // Sell available goods
        for id in 0..self.products.len() {
            if !self.products[id].active() {
                continue;
            }

            self.sell_product(id);
        }

        // Automated products construction
        let mut products_to_builds = Vec::new();
        let mut materials_to_buy = Vec::new();
        for automation in self.automations.iter_mut() {
            if !automation.active() || automation.paused() {
                continue;
            }
            let run = match automation.timer() {
                Some(timer) => {
                    if timer.tick(delta) {
                        true
                    } else {
                        false
                    }
                }
                None => {
                    true
                }
            };

            if !run {
                continue;
            }

            match automation.kind() {
                AutomationKind::Buy(id) => materials_to_buy.push(id),
                AutomationKind::Build(id) => products_to_builds.push(id),
            }
        }
        for id in products_to_builds {
            self.construct_product(id);
        }

        for id in materials_to_buy {
            self.buy_material(id, 1);
        }

        self.win = self.check_conditions(&self.objective.0);

        if self.win {
            return;
        }

        // Make sure we unlock all perks/badges/automations
        // which have their conditions met.
        macro_rules! unlock_perk {
            ($name:ident) => {
                let mut unlocks = Vec::new();
                for (id, inst) in self.$name.iter().enumerate() {
                    if self.check_conditions(&inst.condition()) {
                        unlocks.push(id);
                    }
                }
                for id in unlocks {
                    self.$name[id].unlock();
                }
            };
        }

        unlock_perk!(badges);
        unlock_perk!(perks);
        unlock_perk!(automations);
    }

    pub fn buy_material(&mut self, id: ProductMaterialId, cnt: u32) {
        for _ in 0..cnt {
            let price = self.materials[id].price();
            if self.money >= price {
                self.materials[id].buy();
                self.money -= price;
            }
        }
    }

    pub fn buy_perk(&mut self, id: PerkId) {
        assert!(self.perks[id].unlocked);
        assert!(!self.perks[id].active);

        if !self.check_conditions(self.perks[id].price()) {
            return;
        }

        for price in self.perks[id].price() {
            match price {
                Quantity::Money(money) => {
                    self.money -= money;
                }
                Quantity::Material(id, cnt) => {
                    self.materials[*id].count -= cnt;
                }
                Quantity::Product(id, cnt) => {
                    self.products[*id].count -= cnt;
                }
            }
        }

        self.perks[id].activate();
    }

    pub fn buy_automation(&mut self, id: AutomationId) {
        assert!(self.automations[id].unlocked);
        assert!(!self.automations[id].active);

        if !self.check_conditions(self.automations[id].price()) {
            return;
        }

        for price in self.automations[id].price() {
            match price {
                Quantity::Money(money) => {
                    self.money -= money;
                }
                Quantity::Material(id, cnt) => {
                    self.materials[*id].count -= cnt;
                }
                Quantity::Product(id, cnt) => {
                    self.products[*id].count -= cnt;
                }
            }
        }

        self.automations[id].activate();
    }

    pub fn toggle_automation(&mut self, id: AutomationId) {
        self.automations[id].paused = !self.automations[id].paused;
    }

    pub fn inc_price(&mut self, id: ProductId) {
        if let Some(price) = self.products[id].price {
            self.products[id].price = Some(price + 0.01);
        }
    }

    pub fn dec_price(&mut self, id: ProductId) {
        if let Some(price) = self.products[id].price {
            self.products[id].price = Some((price - 0.01).max(0.01));
        }
    }

    pub fn money(&self) -> f64 {
        self.money
    }

    pub fn win(&self) -> bool {
        self.win
    }
}
