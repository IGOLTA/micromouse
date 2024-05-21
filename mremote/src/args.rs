use clap::{
    Args,
    Parser,
    Subcommand
};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub action_type: ActionType,
}

#[derive(Debug, Subcommand)]
pub enum ActionType {
    ///Permet de changer les parametres
    Set(SetCommand),
    ///Lance une acquisition
    Acquire(AcquireCommand),
    ///Recupere les parametres courants
    Status{},
    ///Trace un diagramme de bode
    Bode(BodeCommand)
}

#[derive(Debug, Args)]
pub struct BodeCommand {
    ///Amplitude des solicitations
    pub amplitude: f32,
    ///w de depart (log)
    pub min_w: f32,
    ///w final (log)
    pub max_w: f32,
    ///Nombre d'echantillons
    pub sample_count: usize,
    ///Nombre d'echantillons par periode de la sinusoide
    #[arg(long, default_value_t=50)]
    pub sine_sample_count: usize,
    ///Nombre de periodes avant regime permanent
    #[arg(short, long, default_value_t=4)]
    pub steady_state_period_count: u32,
    ///Nombre de periodes apres regime permanent
    #[arg(short, long, default_value_t=2)]
    pub period_count: u32,
}

#[derive(Debug, Args)]
pub struct SetCommand {
    #[arg(short, long)]
    pub proportional: Option<f32>,

    #[arg(short, long)]
    pub derivative: Option<f32>,

    #[arg(short, long)]
    pub integral: Option<f32>,

    ///Temps d'echantillonage pour la vitesse et le retour
    #[arg(long)]
    pub feedback_sample_time: Option<u32>,

    #[arg(short, long)]
    pub feedback_enabled: Option<u8>
}

#[derive(Debug, Args)]
pub struct AcquireCommand {
    #[command(subcommand)]
    pub acq_type: AcqType,

    ///Duree de l'acquisition en micro secondes
    pub acquire_duration: u32,

    ///Temps d'echantillonsage en micro secondes
    #[arg(short, long, default_value_t=10000)]
    pub sample_time: u32,
}


#[derive(Debug, Subcommand)]
pub enum AcqType {
    ///Pour faire une entree echelon
    Step(StepCommand),
    ///Pour faire une wntree sinusoidale
    Sine(SineCommand)
}

#[derive(Debug, Args)]
pub struct StepCommand {
    pub value: f32,
}

#[derive(Debug, Args)]
pub struct SineCommand {
    pub amplitude: f32,
    pub pulsation: f32
}